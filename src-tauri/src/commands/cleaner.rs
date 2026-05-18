/// 清理引擎 — Tauri Commands

use crate::models::{CleanError, CleanReport};
use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};

#[tauri::command]
pub async fn clean_items(paths: Vec<String>, safe_mode: bool) -> Result<CleanReport, String> {
    let mut cleaned_count = 0_u64;
    let mut freed_bytes = 0_u64;
    let mut skipped_count = 0_u64;
    let mut errors = Vec::new();

    for path in paths {
        let path_buf = expand_home(&path);

        if !is_safe_clean_target(&path_buf) {
            skipped_count += 1;
            errors.push(CleanError {
                path,
                reason: "Path is outside allowed cleanup areas".to_string(),
            });
            continue;
        }

        let size = path_size(&path_buf);
        let result = if safe_mode {
            trash::delete(&path_buf).map_err(|e| e.to_string())
        } else if path_buf.is_dir() {
            fs::remove_dir_all(&path_buf).map_err(|e| e.to_string())
        } else {
            fs::remove_file(&path_buf).map_err(|e| e.to_string())
        };

        match result {
            Ok(()) => {
                cleaned_count += 1;
                freed_bytes += size;
            }
            Err(reason) => {
                skipped_count += 1;
                errors.push(CleanError { path, reason });
            }
        }
    }

    Ok(CleanReport {
        cleaned_count,
        freed_bytes,
        skipped_count,
        errors,
        snapshot_id: format!("clean-{}", Utc::now().timestamp()),
    })
}

#[tauri::command]
pub async fn get_clean_report(_snapshot_id: String) -> Result<CleanReport, String> {
    Err("Clean report persistence is not implemented yet".to_string())
}

fn expand_home(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(stripped);
        }
    }
    PathBuf::from(path)
}

fn is_safe_clean_target(path: &Path) -> bool {
    let Ok(canonical) = path.canonicalize() else {
        return false;
    };
    let text = canonical.to_string_lossy();
    let Some(home) = dirs::home_dir() else {
        return false;
    };
    let home_text = home.to_string_lossy();

    text.starts_with(&format!("{}/Library/Caches", home_text))
        || text.starts_with(&format!("{}/Library/Logs", home_text))
        || text.starts_with(&format!("{}/Library/Developer/Xcode", home_text))
        || text.starts_with(&format!("{}/Downloads", home_text))
        || text.starts_with(&format!("{}/.Trash", home_text))
        || text.starts_with("/Library/Caches")
        || text.starts_with("/Library/Logs")
}

fn path_size(path: &Path) -> u64 {
    if path.is_file() {
        return fs::metadata(path).map(|meta| meta.len()).unwrap_or(0);
    }

    if !path.is_dir() {
        return 0;
    }

    walkdir::WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter_map(|entry| entry.metadata().ok())
        .filter(|metadata| metadata.is_file())
        .map(|metadata| metadata.len())
        .sum()
}
