//! 空间地图 — 对指定目录做一次只读、同文件系统的递归汇总。

use crate::models::{SpaceMapEntry, SpaceMapResult};
use chrono::{DateTime, Utc};
use std::collections::{BTreeMap, HashSet};
use std::fs::{self, Metadata};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use walkdir::WalkDir;

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

#[derive(Default)]
struct EntryAggregate {
    logical_size: u64,
    allocated_size: u64,
    file_count: u64,
    directory_count: u64,
}

#[derive(Default)]
pub struct SpaceMapState {
    generation: Arc<AtomicU64>,
}

#[tauri::command]
pub async fn analyze_space_map(
    path: Option<String>,
    state: tauri::State<'_, SpaceMapState>,
) -> Result<SpaceMapResult, String> {
    let generation = state.generation.fetch_add(1, Ordering::Relaxed) + 1;
    let current_generation = Arc::clone(&state.generation);
    tokio::task::spawn_blocking(move || {
        let requested = match path {
            Some(value) if !value.trim().is_empty() => expand_home(value.trim()),
            _ => dirs::home_dir().ok_or_else(|| "无法确定当前用户目录".to_string())?,
        };
        analyze_directory(&requested, Some((&current_generation, generation)))
    })
    .await
    .map_err(|error| format!("空间分析任务异常结束：{error}"))?
}

#[tauri::command]
pub fn cancel_space_map(state: tauri::State<'_, SpaceMapState>) {
    state.generation.fetch_add(1, Ordering::Relaxed);
}

#[tauri::command]
pub async fn choose_space_map_directory() -> Result<Option<String>, String> {
    tokio::task::spawn_blocking(|| {
        let output = std::process::Command::new("/usr/bin/osascript")
            .args([
                "-e",
                "POSIX path of (choose folder with prompt \"选择要分析的文件夹\")",
            ])
            .output()
            .map_err(|error| format!("无法打开文件夹选择器：{error}"))?;

        if !output.status.success() {
            // AppleScript 在用户取消选择时返回非零状态；取消不是错误。
            return Ok(None);
        }

        let selected = String::from_utf8_lossy(&output.stdout)
            .trim()
            .trim_end_matches('/')
            .to_string();
        Ok((!selected.is_empty()).then_some(selected))
    })
    .await
    .map_err(|error| format!("文件夹选择任务异常结束：{error}"))?
}

fn analyze_directory(
    requested: &Path,
    cancellation: Option<(&AtomicU64, u64)>,
) -> Result<SpaceMapResult, String> {
    let started_at = Instant::now();
    let root = requested
        .canonicalize()
        .map_err(|error| format!("无法访问 {}：{error}", requested.display()))?;
    if !root.is_dir() {
        return Err(format!("{} 不是文件夹", root.display()));
    }

    let mut aggregates: BTreeMap<PathBuf, EntryAggregate> = BTreeMap::new();
    let mut skipped_items = 0_u64;
    let mut hard_link_duplicates = 0_u64;
    let mut symlink_count = 0_u64;
    let mut seen_hard_links: HashSet<(u64, u64)> = HashSet::new();

    for item in WalkDir::new(&root)
        .follow_links(false)
        .same_file_system(true)
        .into_iter()
    {
        if cancellation.is_some_and(|(current, generation)| {
            current.load(Ordering::Relaxed) != generation
        }) {
            return Err("空间分析已取消".to_string());
        }
        let entry = match item {
            Ok(entry) => entry,
            Err(_) => {
                skipped_items += 1;
                continue;
            }
        };
        if entry.depth() == 0 {
            continue;
        }

        if entry.file_type().is_symlink() {
            symlink_count += 1;
            continue;
        }

        let Some(first_component) = entry
            .path()
            .strip_prefix(&root)
            .ok()
            .and_then(|relative| relative.components().next())
        else {
            skipped_items += 1;
            continue;
        };
        let direct_child = root.join(first_component.as_os_str());
        let aggregate = aggregates.entry(direct_child).or_default();

        if entry.file_type().is_dir() {
            aggregate.directory_count += 1;
            continue;
        }
        if !entry.file_type().is_file() {
            continue;
        }

        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(_) => {
                skipped_items += 1;
                continue;
            }
        };

        if is_duplicate_hard_link(&metadata, &mut seen_hard_links) {
            hard_link_duplicates += 1;
            continue;
        }

        aggregate.file_count += 1;
        aggregate.logical_size = aggregate.logical_size.saturating_add(metadata.len());
        aggregate.allocated_size = aggregate
            .allocated_size
            .saturating_add(allocated_bytes(&metadata));
    }

    let mut entries = Vec::with_capacity(aggregates.len());
    for (path, aggregate) in aggregates {
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(_) => {
                skipped_items += 1;
                continue;
            }
        };
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        entries.push(SpaceMapEntry {
            is_package: metadata.is_dir() && is_package_directory(&path),
            is_cloud_placeholder: is_cloud_placeholder(&path, &metadata),
            name,
            path: path.to_string_lossy().into_owned(),
            logical_size: aggregate.logical_size,
            allocated_size: aggregate.allocated_size,
            file_count: aggregate.file_count,
            directory_count: aggregate.directory_count,
            modified_at: modified_timestamp(&metadata),
            is_dir: metadata.is_dir(),
        });
    }
    entries.sort_by(|left, right| {
        right
            .logical_size
            .cmp(&left.logical_size)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });

    Ok(SpaceMapResult {
        root_path: root.to_string_lossy().into_owned(),
        display_path: display_path(&root),
        logical_size: entries.iter().map(|entry| entry.logical_size).sum(),
        allocated_size: entries.iter().map(|entry| entry.allocated_size).sum(),
        file_count: entries.iter().map(|entry| entry.file_count).sum(),
        directory_count: entries.iter().map(|entry| entry.directory_count).sum(),
        entries,
        skipped_items,
        hard_link_duplicates,
        symlink_count,
        scan_duration_ms: started_at.elapsed().as_millis() as u64,
    })
}

fn expand_home(path: &str) -> PathBuf {
    if path == "~" {
        return dirs::home_dir().unwrap_or_else(|| PathBuf::from(path));
    }
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}

#[cfg(unix)]
fn allocated_bytes(metadata: &Metadata) -> u64 {
    metadata.blocks().saturating_mul(512)
}

#[cfg(not(unix))]
fn allocated_bytes(metadata: &Metadata) -> u64 {
    metadata.len()
}

#[cfg(unix)]
fn is_duplicate_hard_link(metadata: &Metadata, seen: &mut HashSet<(u64, u64)>) -> bool {
    metadata.nlink() > 1 && !seen.insert((metadata.dev(), metadata.ino()))
}

#[cfg(not(unix))]
fn is_duplicate_hard_link(_metadata: &Metadata, _seen: &mut HashSet<(u64, u64)>) -> bool {
    false
}

fn modified_timestamp(metadata: &Metadata) -> Option<String> {
    metadata
        .modified()
        .ok()
        .map(DateTime::<Utc>::from)
        .map(|value| value.to_rfc3339())
}

fn is_package_directory(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.to_ascii_lowercase())
            .as_deref(),
        Some(
            "app"
                | "bundle"
                | "framework"
                | "photoslibrary"
                | "photolibrary"
                | "imovielibrary"
                | "logicx"
                | "rtfd"
        )
    )
}

fn is_cloud_placeholder(path: &Path, metadata: &Metadata) -> bool {
    let placeholder_extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("icloud"));
    placeholder_extension
        || (metadata.is_file() && metadata.len() > 0 && allocated_bytes(metadata) == 0)
}

fn display_path(path: &Path) -> String {
    if let Some(home) = dirs::home_dir() {
        if path == home {
            return "~".to_string();
        }
        if let Ok(relative) = path.strip_prefix(&home) {
            return format!("~/{}", relative.display());
        }
    }
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture_dir(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("cleanmac-space-map-{name}-{nonce}"));
        fs::create_dir_all(&root).expect("create fixture");
        root
    }

    #[test]
    fn aggregates_direct_children_and_files() {
        let root = fixture_dir("aggregate");
        fs::create_dir_all(root.join("Projects/build")).expect("create dirs");
        fs::write(root.join("Projects/source.txt"), vec![1_u8; 11]).expect("write source");
        fs::write(root.join("Projects/build/app.bin"), vec![2_u8; 29]).expect("write build");
        fs::write(root.join("readme.md"), vec![3_u8; 7]).expect("write readme");

        let result = analyze_directory(&root, None).expect("analyze fixture");
        assert_eq!(result.logical_size, 47);
        assert_eq!(result.file_count, 3);
        assert_eq!(result.entries.len(), 2);
        assert_eq!(result.entries[0].name, "Projects");
        assert_eq!(result.entries[0].logical_size, 40);

        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[cfg(unix)]
    #[test]
    fn counts_hard_link_content_once() {
        let root = fixture_dir("hard-link");
        let original = root.join("original.bin");
        let mut file = fs::File::create(&original).expect("create original");
        file.write_all(&[7_u8; 16]).expect("write original");
        fs::hard_link(&original, root.join("copy.bin")).expect("create hard link");

        let result = analyze_directory(&root, None).expect("analyze fixture");
        assert_eq!(result.logical_size, 16);
        assert_eq!(result.file_count, 1);
        assert_eq!(result.hard_link_duplicates, 1);

        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[cfg(unix)]
    #[test]
    fn does_not_follow_symbolic_links_outside_scope() {
        use std::os::unix::fs::symlink;

        let root = fixture_dir("symbolic-link");
        let outside = fixture_dir("outside");
        fs::write(outside.join("private.bin"), vec![4_u8; 64]).expect("write outside file");
        symlink(&outside, root.join("linked-outside")).expect("create symbolic link");

        let result = analyze_directory(&root, None).expect("analyze fixture");
        assert_eq!(result.logical_size, 0);
        assert_eq!(result.symlink_count, 1);
        assert!(result.entries.is_empty());

        fs::remove_dir_all(root).expect("remove fixture");
        fs::remove_dir_all(outside).expect("remove outside fixture");
    }

    #[test]
    fn stops_when_a_newer_analysis_replaces_the_request() {
        let root = fixture_dir("cancelled");
        fs::write(root.join("file.bin"), vec![9_u8; 12]).expect("write fixture");
        let generation = AtomicU64::new(2);

        let error = analyze_directory(&root, Some((&generation, 1))).expect_err("cancel scan");
        assert_eq!(error, "空间分析已取消");

        fs::remove_dir_all(root).expect("remove fixture");
    }
}
