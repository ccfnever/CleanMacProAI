/// 软件卸载 — Tauri Commands

use crate::models::{CleanError, CleanReport, FileInfo, InstalledApp};
use chrono::Utc;
use plist::Value;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const MAX_RELATED_PREVIEW_FILES: usize = 24;

#[tauri::command]
pub async fn list_installed_apps() -> Result<Vec<InstalledApp>, String> {
    let mut roots = vec![PathBuf::from("/Applications")];
    if let Some(home) = dirs::home_dir() {
        roots.push(home.join("Applications"));
    }

    let mut apps = Vec::new();
    for root in roots {
        if !root.exists() {
            continue;
        }

        let Ok(entries) = fs::read_dir(root) else {
            continue;
        };

        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("app") {
                continue;
            }

            if let Some(app) = read_app_bundle(&path, false) {
                apps.push(app);
            }
        }
    }

    apps.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    Ok(apps)
}

#[tauri::command]
pub async fn inspect_installed_app(bundle_id: String) -> Result<InstalledApp, String> {
    find_app_by_bundle_id(&bundle_id, true)
        .ok_or_else(|| format!("Application '{}' not found", bundle_id))
}

#[tauri::command]
pub async fn uninstall_app(bundle_id: String, move_to_trash: bool) -> Result<CleanReport, String> {
    let app = find_app_by_bundle_id(&bundle_id, true)
        .ok_or_else(|| "Application not found".to_string())?;

    if app.is_system_app {
        return Err("Refusing to uninstall a system application".to_string());
    }

    let mut targets = vec![app.app_path.clone()];
    targets.extend(app.related_files.iter().map(|file| file.path.clone()));
    let mut seen = HashSet::new();
    targets.retain(|target| seen.insert(target.clone()));

    let mut cleaned_count = 0_u64;
    let mut freed_bytes = 0_u64;
    let mut skipped_count = 0_u64;
    let mut errors = Vec::new();

    for target in targets {
        let path = expand_home(&target);
        if !is_safe_uninstall_target(&path) {
            skipped_count += 1;
            errors.push(CleanError {
                path: target,
                reason: "Path is outside uninstall safety boundaries".to_string(),
            });
            continue;
        }

        let size = path_size(&path);
        let count = path_count_for_report(&path);
        let result = if move_to_trash {
            trash::delete(&path).map_err(|e| e.to_string())
        } else if path.is_dir() {
            fs::remove_dir_all(&path).map_err(|e| e.to_string())
        } else {
            fs::remove_file(&path).map_err(|e| e.to_string())
        };

        match result {
            Ok(()) => {
                cleaned_count += count;
                freed_bytes += size;
            }
            Err(reason) => {
                skipped_count += 1;
                errors.push(CleanError { path: target, reason });
            }
        }
    }

    Ok(CleanReport {
        cleaned_count,
        freed_bytes,
        skipped_count,
        errors,
        snapshot_id: format!("uninstall-{}", Utc::now().timestamp()),
    })
}

fn find_app_by_bundle_id(bundle_id: &str, include_details: bool) -> Option<InstalledApp> {
    application_roots()
        .into_iter()
        .filter(|root| root.exists())
        .filter_map(|root| fs::read_dir(root).ok())
        .flat_map(|entries| entries.filter_map(Result::ok))
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("app"))
        .filter_map(|path| read_app_bundle(&path, include_details))
        .find(|app| app.bundle_id == bundle_id)
}

fn application_roots() -> Vec<PathBuf> {
    let mut roots = vec![PathBuf::from("/Applications")];
    if let Some(home) = dirs::home_dir() {
        roots.push(home.join("Applications"));
    }
    roots
}

fn read_app_bundle(path: &Path, include_details: bool) -> Option<InstalledApp> {
    let info_plist = path.join("Contents/Info.plist");
    let value = Value::from_file(info_plist).ok()?;
    let dictionary = value.as_dictionary()?;

    let name = dictionary
        .get("CFBundleDisplayName")
        .and_then(Value::as_string)
        .or_else(|| dictionary.get("CFBundleName").and_then(Value::as_string))
        .map(ToOwned::to_owned)
        .or_else(|| {
            path.file_stem()
                .and_then(|name| name.to_str())
                .map(ToOwned::to_owned)
        })?;

    let bundle_id = dictionary
        .get("CFBundleIdentifier")
        .and_then(Value::as_string)
        .unwrap_or("unknown.bundle")
        .to_string();

    let app_size = if include_details { path_size(path) } else { 0 };
    let related = if include_details {
        related_app_data(&name, &bundle_id)
    } else {
        RelatedAppData {
            total_size: 0,
            total_count: 0,
            preview_files: Vec::new(),
        }
    };
    let app_path = path.to_string_lossy().to_string();
    let is_system_app = app_path.starts_with("/System/");

    Some(InstalledApp {
        name,
        bundle_id,
        app_path,
        app_size,
        related_size: related.total_size,
        related_count: related.total_count,
        related_files: related.preview_files,
        is_system_app,
    })
}

struct RelatedAppData {
    total_size: u64,
    total_count: u64,
    preview_files: Vec<FileInfo>,
}

fn related_app_data(name: &str, bundle_id: &str) -> RelatedAppData {
    let Some(home) = dirs::home_dir() else {
        return RelatedAppData {
            total_size: 0,
            total_count: 0,
            preview_files: Vec::new(),
        };
    };

    let keywords = app_match_keywords(name, bundle_id);
    let candidate_paths = related_library_matches(&home, &keywords);

    let mut total_size = 0_u64;
    let mut total_count = 0_u64;
    let mut preview_files = Vec::new();

    for path in candidate_paths {
        if !path.exists() {
            continue;
        }

        let size = path_size(&path);
        let count = path_count_for_report(&path);
        total_size += size;
        total_count += count;

        if preview_files.len() < MAX_RELATED_PREVIEW_FILES {
            preview_files.push(FileInfo {
                path: display_path(&path),
                size,
                modified_at: fs::metadata(&path)
                    .ok()
                    .and_then(|metadata| metadata.modified().ok())
                    .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|duration| duration.as_secs().to_string()),
            });
        }
    }

    RelatedAppData {
        total_size,
        total_count,
        preview_files,
    }
}

fn app_match_keywords(name: &str, bundle_id: &str) -> Vec<String> {
    let mut keywords = Vec::new();
    add_keyword(&mut keywords, name);
    add_keyword(&mut keywords, bundle_id);

    for segment in bundle_id.split(['.', '-', '_']) {
        let segment = segment.trim();
        if segment.len() >= 3 && !matches!(segment, "app" | "com" | "net" | "org" | "www") {
            add_keyword(&mut keywords, segment);
        }
    }

    keywords
}

fn add_keyword(keywords: &mut Vec<String>, value: &str) {
    let normalized = value.trim().to_lowercase();
    if normalized.len() >= 2 && !keywords.contains(&normalized) {
        keywords.push(normalized);
    }
}

fn related_library_matches(home: &Path, keywords: &[String]) -> Vec<PathBuf> {
    let roots = [
        home.join("Library/Caches"),
        home.join("Library/Logs"),
        home.join("Library/Preferences"),
        home.join("Library/Application Support"),
    ];
    let mut paths = Vec::new();
    let mut seen = HashSet::new();

    for root in roots {
        let Ok(entries) = fs::read_dir(root) else {
            continue;
        };

        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            let normalized = file_name.to_lowercase();
            if keywords.iter().any(|keyword| normalized.contains(keyword))
                && seen.insert(path.clone())
            {
                paths.push(path);
            }
        }
    }

    paths
}

fn path_size(path: &Path) -> u64 {
    if path.is_file() {
        return fs::metadata(path).map(|meta| meta.len()).unwrap_or(0);
    }

    if !path.is_dir() {
        return 0;
    }

    du_size(path).unwrap_or(0)
}

fn du_size(path: &Path) -> Option<u64> {
    Command::new("/usr/bin/du")
        .args(["-sk"])
        .arg(path)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| {
            String::from_utf8(output.stdout)
                .ok()
                .and_then(|stdout| stdout.split_whitespace().next().map(str::to_string))
        })
        .and_then(|kilobytes| kilobytes.parse::<u64>().ok())
        .map(|kilobytes| kilobytes.saturating_mul(1024))
}

fn path_count_for_report(path: &Path) -> u64 {
    let _ = path;
    1
}

fn expand_home(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(stripped);
        }
    }
    PathBuf::from(path)
}

fn display_path(path: &Path) -> String {
    let absolute = path.to_string_lossy().to_string();
    let Some(home) = dirs::home_dir() else {
        return absolute;
    };
    let home_text = home.to_string_lossy();
    absolute
        .strip_prefix(home_text.as_ref())
        .map(|rest| format!("~{}", rest))
        .unwrap_or(absolute)
}

fn is_safe_uninstall_target(path: &Path) -> bool {
    let Ok(canonical) = path.canonicalize() else {
        return false;
    };
    let text = canonical.to_string_lossy();
    let Some(home) = dirs::home_dir() else {
        return false;
    };
    let home_text = home.to_string_lossy();

    text.starts_with("/Applications/")
        || text.starts_with(&format!("{}/Applications/", home_text))
        || text.starts_with(&format!("{}/Library/Caches/", home_text))
        || text.starts_with(&format!("{}/Library/Logs/", home_text))
        || text.starts_with(&format!("{}/Library/Preferences/", home_text))
        || text.starts_with(&format!("{}/Library/Application Support/", home_text))
        || text.starts_with(&format!("{}/Library/Saved Application State/", home_text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_size_reports_existing_application_bundle() {
        let app_path = application_roots()
            .into_iter()
            .filter_map(|root| fs::read_dir(root).ok())
            .flat_map(|entries| entries.filter_map(Result::ok))
            .map(|entry| entry.path())
            .find(|path| path.extension().and_then(|ext| ext.to_str()) == Some("app"));

        if let Some(path) = app_path {
            assert!(
                path_size(&path) > 0,
                "expected non-zero size for {}",
                path.display()
            );
        }
    }
}
