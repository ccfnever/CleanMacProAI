//! 空间地图 — 并行查找指定目录中的大文件，并持续发布目录汇总。

use crate::models::{SpaceMapEntry, SpaceMapProgress, SpaceMapResult};
use chrono::{DateTime, Utc};
use std::collections::HashSet;
use std::fs::{self, Metadata};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};
use walkdir::WalkDir;

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

const DEFAULT_MINIMUM_FILE_SIZE: u64 = 100 * 1024 * 1024;
const MINIMUM_ALLOWED_FILE_SIZE: u64 = 1024 * 1024;
const MAXIMUM_ALLOWED_FILE_SIZE: u64 = 10 * 1024 * 1024 * 1024;
const MAX_SCAN_WORKERS: usize = 4;
const PUBLISH_ITEM_INTERVAL: u64 = 160;
const PUBLISH_TIME_INTERVAL: Duration = Duration::from_millis(90);
const MAX_PROGRESS_ENTRIES: usize = 32;

#[derive(Default)]
pub struct SpaceMapState {
    generation: Arc<AtomicU64>,
    progress: Arc<Mutex<SpaceMapProgress>>,
}

#[derive(Clone)]
struct ScanContext {
    generation: Arc<AtomicU64>,
    request_generation: u64,
    progress: Arc<Mutex<SpaceMapProgress>>,
    seen_hard_links: Arc<Mutex<HashSet<(u64, u64)>>>,
    started_at: Instant,
}

#[derive(Default)]
struct ProgressDelta {
    scanned_file_count: u64,
    scanned_directory_count: u64,
    matched_file_count: u64,
    matched_logical_size: u64,
    matched_allocated_size: u64,
    ignored_file_count: u64,
    ignored_logical_size: u64,
    skipped_items: u64,
    hard_link_duplicates: u64,
    symlink_count: u64,
    entry_logical_size: u64,
    entry_allocated_size: u64,
    entry_file_count: u64,
    entry_directory_count: u64,
    inspected_items: u64,
}

#[tauri::command]
pub async fn analyze_space_map(
    path: Option<String>,
    minimum_file_size: Option<u64>,
    state: tauri::State<'_, SpaceMapState>,
) -> Result<SpaceMapResult, String> {
    let requested = match path {
        Some(value) if !value.trim().is_empty() => expand_home(value.trim()),
        _ => dirs::home_dir().ok_or_else(|| "无法确定当前用户目录".to_string())?,
    };
    let threshold = minimum_file_size
        .unwrap_or(DEFAULT_MINIMUM_FILE_SIZE)
        .clamp(MINIMUM_ALLOWED_FILE_SIZE, MAXIMUM_ALLOWED_FILE_SIZE);
    let request_generation = state.generation.fetch_add(1, Ordering::Relaxed) + 1;
    let context = ScanContext {
        generation: Arc::clone(&state.generation),
        request_generation,
        progress: Arc::clone(&state.progress),
        seen_hard_links: Arc::new(Mutex::new(HashSet::new())),
        started_at: Instant::now(),
    };

    {
        let mut progress = lock(&state.progress);
        *progress = SpaceMapProgress {
            is_scanning: true,
            root_path: requested.to_string_lossy().into_owned(),
            display_path: display_path(&requested),
            minimum_file_size: threshold,
            ..SpaceMapProgress::default()
        };
    }

    let scan_context = context.clone();
    let scan_result = tokio::task::spawn_blocking(move || {
        analyze_directory(&requested, threshold, &scan_context)
    })
    .await
    .map_err(|error| format!("空间分析任务异常结束：{error}"))?;

    if context.generation.load(Ordering::Relaxed) == request_generation {
        let mut progress = lock(&context.progress);
        progress.is_scanning = false;
        progress.current_path = None;
        progress.elapsed_ms = context.started_at.elapsed().as_millis() as u64;
    }
    scan_result
}

#[tauri::command]
pub fn get_space_map_progress(state: tauri::State<'_, SpaceMapState>) -> SpaceMapProgress {
    let mut snapshot = lock(&state.progress).clone();
    snapshot.entries.sort_by(|left, right| {
        right
            .logical_size
            .cmp(&left.logical_size)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
    snapshot.entries.truncate(MAX_PROGRESS_ENTRIES);
    snapshot
}

#[tauri::command]
pub fn cancel_space_map(state: tauri::State<'_, SpaceMapState>) {
    state.generation.fetch_add(1, Ordering::Relaxed);
    lock(&state.progress).is_scanning = false;
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
    minimum_file_size: u64,
    context: &ScanContext,
) -> Result<SpaceMapResult, String> {
    let root = requested
        .canonicalize()
        .map_err(|error| format!("无法访问 {}：{error}", requested.display()))?;
    if !root.is_dir() {
        return Err(format!("{} 不是文件夹", root.display()));
    }

    {
        let mut progress = lock(&context.progress);
        progress.root_path = root.to_string_lossy().into_owned();
        progress.display_path = display_path(&root);
    }

    let mut root_read_errors = 0_u64;
    let children = fs::read_dir(&root)
        .map_err(|error| format!("无法读取 {}：{error}", root.display()))?
        .filter_map(|entry| match entry {
            Ok(entry) => Some(entry.path()),
            Err(_) => {
                root_read_errors += 1;
                None
            }
        })
        .collect::<Vec<_>>();
    if root_read_errors > 0 {
        lock(&context.progress).skipped_items += root_read_errors;
    }

    let next_child = AtomicUsize::new(0);
    let worker_count = available_worker_count(children.len());
    std::thread::scope(|scope| {
        for _ in 0..worker_count {
            let next_child = &next_child;
            let children = &children;
            let context = context.clone();
            scope.spawn(move || loop {
                if is_cancelled(&context) {
                    break;
                }
                let index = next_child.fetch_add(1, Ordering::Relaxed);
                let Some(child) = children.get(index) else {
                    break;
                };
                scan_direct_child(child, minimum_file_size, &context);
            });
        }
    });

    if is_cancelled(context) {
        return Err("空间分析已取消".to_string());
    }

    let snapshot = lock(&context.progress).clone();
    let mut entries = snapshot
        .entries
        .into_iter()
        .filter(|entry| entry.logical_size > 0)
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| {
        right
            .logical_size
            .cmp(&left.logical_size)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });

    Ok(SpaceMapResult {
        root_path: root.to_string_lossy().into_owned(),
        display_path: display_path(&root),
        logical_size: snapshot.matched_logical_size,
        allocated_size: snapshot.matched_allocated_size,
        file_count: snapshot.matched_file_count,
        directory_count: snapshot.scanned_directory_count,
        scanned_file_count: snapshot.scanned_file_count,
        ignored_file_count: snapshot.ignored_file_count,
        ignored_logical_size: snapshot.ignored_logical_size,
        minimum_file_size,
        entries,
        skipped_items: snapshot.skipped_items,
        hard_link_duplicates: snapshot.hard_link_duplicates,
        symlink_count: snapshot.symlink_count,
        scan_duration_ms: context.started_at.elapsed().as_millis() as u64,
    })
}

fn scan_direct_child(
    direct_child: &Path,
    minimum_file_size: u64,
    context: &ScanContext,
) {
    let metadata = match fs::symlink_metadata(direct_child) {
        Ok(metadata) => metadata,
        Err(_) => {
            lock(&context.progress).skipped_items += 1;
            return;
        }
    };
    if metadata.file_type().is_symlink() {
        lock(&context.progress).symlink_count += 1;
        return;
    }

    let entry_seed = make_entry(direct_child, &metadata);
    let mut delta = ProgressDelta::default();
    let mut last_publish = Instant::now();

    if metadata.is_file() {
        inspect_file(&metadata, minimum_file_size, context, &mut delta);
        delta.inspected_items += 1;
        publish_delta(context, &entry_seed, &mut delta, direct_child);
        return;
    }
    if !metadata.is_dir() {
        return;
    }

    for item in WalkDir::new(direct_child)
        .follow_links(false)
        .same_file_system(true)
        .into_iter()
    {
        if is_cancelled(context) {
            return;
        }
        let entry = match item {
            Ok(entry) => entry,
            Err(_) => {
                delta.skipped_items += 1;
                delta.inspected_items += 1;
                continue;
            }
        };

        delta.inspected_items += 1;
        if entry.file_type().is_symlink() {
            delta.symlink_count += 1;
        } else if entry.file_type().is_dir() {
            delta.scanned_directory_count += 1;
            delta.entry_directory_count += 1;
        } else if entry.file_type().is_file() {
            match entry.metadata() {
                Ok(file_metadata) => {
                    inspect_file(&file_metadata, minimum_file_size, context, &mut delta)
                }
                Err(_) => delta.skipped_items += 1,
            }
        }

        if delta.inspected_items >= PUBLISH_ITEM_INTERVAL
            || last_publish.elapsed() >= PUBLISH_TIME_INTERVAL
        {
            publish_delta(context, &entry_seed, &mut delta, entry.path());
            last_publish = Instant::now();
        }
    }
    publish_delta(context, &entry_seed, &mut delta, direct_child);
}

fn inspect_file(
    metadata: &Metadata,
    minimum_file_size: u64,
    context: &ScanContext,
    delta: &mut ProgressDelta,
) {
    delta.scanned_file_count += 1;
    if is_duplicate_hard_link(metadata, &context.seen_hard_links) {
        delta.hard_link_duplicates += 1;
        return;
    }

    let size = metadata.len();
    if size < minimum_file_size {
        delta.ignored_file_count += 1;
        delta.ignored_logical_size = delta.ignored_logical_size.saturating_add(size);
        return;
    }

    let allocated = allocated_bytes(metadata);
    delta.matched_file_count += 1;
    delta.matched_logical_size = delta.matched_logical_size.saturating_add(size);
    delta.matched_allocated_size = delta.matched_allocated_size.saturating_add(allocated);
    delta.entry_file_count += 1;
    delta.entry_logical_size = delta.entry_logical_size.saturating_add(size);
    delta.entry_allocated_size = delta.entry_allocated_size.saturating_add(allocated);
}

fn publish_delta(
    context: &ScanContext,
    seed: &SpaceMapEntry,
    delta: &mut ProgressDelta,
    current_path: &Path,
) {
    if delta.inspected_items == 0 {
        return;
    }
    let mut progress = lock(&context.progress);
    if is_cancelled(context) {
        *delta = ProgressDelta::default();
        return;
    }
    progress.current_path = Some(display_path(current_path));
    progress.scanned_file_count += delta.scanned_file_count;
    progress.scanned_directory_count += delta.scanned_directory_count;
    progress.matched_file_count += delta.matched_file_count;
    progress.matched_logical_size = progress
        .matched_logical_size
        .saturating_add(delta.matched_logical_size);
    progress.matched_allocated_size = progress
        .matched_allocated_size
        .saturating_add(delta.matched_allocated_size);
    progress.ignored_file_count += delta.ignored_file_count;
    progress.ignored_logical_size = progress
        .ignored_logical_size
        .saturating_add(delta.ignored_logical_size);
    progress.skipped_items += delta.skipped_items;
    progress.hard_link_duplicates += delta.hard_link_duplicates;
    progress.symlink_count += delta.symlink_count;
    progress.elapsed_ms = context.started_at.elapsed().as_millis() as u64;

    if delta.entry_logical_size > 0 || delta.entry_directory_count > 0 {
        if let Some(entry) = progress.entries.iter_mut().find(|entry| entry.path == seed.path) {
            entry.logical_size = entry.logical_size.saturating_add(delta.entry_logical_size);
            entry.allocated_size = entry
                .allocated_size
                .saturating_add(delta.entry_allocated_size);
            entry.file_count += delta.entry_file_count;
            entry.directory_count += delta.entry_directory_count;
        } else {
            let mut entry = seed.clone();
            entry.logical_size = delta.entry_logical_size;
            entry.allocated_size = delta.entry_allocated_size;
            entry.file_count = delta.entry_file_count;
            entry.directory_count = delta.entry_directory_count;
            progress.entries.push(entry);
        }
    }
    *delta = ProgressDelta::default();
}

fn make_entry(path: &Path, metadata: &Metadata) -> SpaceMapEntry {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());
    SpaceMapEntry {
        name,
        path: path.to_string_lossy().into_owned(),
        logical_size: 0,
        allocated_size: 0,
        file_count: 0,
        directory_count: 0,
        modified_at: modified_timestamp(metadata),
        is_dir: metadata.is_dir(),
        is_package: metadata.is_dir() && is_package_directory(path),
        is_cloud_placeholder: is_cloud_placeholder(path, metadata),
    }
}

fn available_worker_count(task_count: usize) -> usize {
    let available = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(2);
    task_count.max(1).min(available).min(MAX_SCAN_WORKERS)
}

fn is_cancelled(context: &ScanContext) -> bool {
    context.generation.load(Ordering::Relaxed) != context.request_generation
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
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
fn is_duplicate_hard_link(
    metadata: &Metadata,
    seen: &Mutex<HashSet<(u64, u64)>>,
) -> bool {
    metadata.nlink() > 1 && !lock(seen).insert((metadata.dev(), metadata.ino()))
}

#[cfg(not(unix))]
fn is_duplicate_hard_link(
    _metadata: &Metadata,
    _seen: &Mutex<HashSet<(u64, u64)>>,
) -> bool {
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

    fn test_context(generation: u64) -> ScanContext {
        ScanContext {
            generation: Arc::new(AtomicU64::new(generation)),
            request_generation: 1,
            progress: Arc::new(Mutex::new(SpaceMapProgress::default())),
            seen_hard_links: Arc::new(Mutex::new(HashSet::new())),
            started_at: Instant::now(),
        }
    }

    #[test]
    fn keeps_only_files_at_or_above_threshold() {
        let root = fixture_dir("threshold");
        fs::create_dir_all(root.join("Projects/build")).expect("create dirs");
        fs::write(root.join("Projects/small.txt"), vec![1_u8; 11]).expect("write small");
        fs::write(root.join("Projects/build/large.bin"), vec![2_u8; 29]).expect("write large");
        fs::write(root.join("readme.md"), vec![3_u8; 7]).expect("write readme");

        let result = analyze_directory(&root, 20, &test_context(1)).expect("analyze fixture");
        assert_eq!(result.logical_size, 29);
        assert_eq!(result.file_count, 1);
        assert_eq!(result.scanned_file_count, 3);
        assert_eq!(result.ignored_file_count, 2);
        assert_eq!(result.ignored_logical_size, 18);
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].name, "Projects");

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

        let result = analyze_directory(&root, 1, &test_context(1)).expect("analyze fixture");
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

        let result = analyze_directory(&root, 1, &test_context(1)).expect("analyze fixture");
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

        let error = analyze_directory(&root, 1, &test_context(2)).expect_err("cancel scan");
        assert_eq!(error, "空间分析已取消");

        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[test]
    fn stale_worker_cannot_write_into_a_new_scan() {
        let context = test_context(2);
        let seed = SpaceMapEntry {
            name: "Old".to_string(),
            path: "/tmp/old".to_string(),
            logical_size: 0,
            allocated_size: 0,
            file_count: 0,
            directory_count: 0,
            modified_at: None,
            is_dir: true,
            is_package: false,
            is_cloud_placeholder: false,
        };
        let mut delta = ProgressDelta {
            matched_file_count: 1,
            matched_logical_size: 64,
            entry_logical_size: 64,
            entry_file_count: 1,
            inspected_items: 1,
            ..ProgressDelta::default()
        };

        publish_delta(&context, &seed, &mut delta, Path::new("/tmp/old/file"));

        let progress = lock(&context.progress);
        assert_eq!(progress.matched_file_count, 0);
        assert!(progress.entries.is_empty());
    }

    #[test]
    fn progress_contains_partial_directory_tiles() {
        let root = fixture_dir("progress");
        fs::create_dir_all(root.join("Media")).expect("create dirs");
        fs::write(root.join("Media/movie.mov"), vec![5_u8; 48]).expect("write large");
        let context = test_context(1);

        analyze_directory(&root, 20, &context).expect("analyze fixture");
        let progress = lock(&context.progress);
        assert_eq!(progress.matched_file_count, 1);
        assert_eq!(progress.entries.len(), 1);
        assert_eq!(progress.entries[0].name, "Media");
        assert_eq!(progress.entries[0].logical_size, 48);

        fs::remove_dir_all(root).expect("remove fixture");
    }

}
