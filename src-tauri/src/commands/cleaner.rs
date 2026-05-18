/// 清理引擎 — Tauri Commands

use crate::models::{CleanError, CleanReport};
use crate::rules::{load_rules, CategoryRule};
use chrono::Utc;
use glob::glob;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const RULES_YAML: &str = include_str!("../rules/cleanup_rules.yaml");

#[tauri::command]
pub async fn clean_items(paths: Vec<String>, safe_mode: bool) -> Result<CleanReport, String> {
    clean_paths(paths, safe_mode)
}

#[tauri::command]
pub async fn clean_categories(
    category_ids: Vec<String>,
    safe_mode: bool,
) -> Result<CleanReport, String> {
    let rules = load_rules(RULES_YAML)?;
    let mut paths = Vec::new();
    let mut skipped_errors = Vec::new();

    for category_id in category_ids {
        let Some(rule) = rules.categories.get(&category_id) else {
            skipped_errors.push(CleanError {
                path: category_id,
                reason: "Unknown cleanup category".to_string(),
            });
            continue;
        };

        if rule.risk == "high" {
            skipped_errors.push(CleanError {
                path: category_id,
                reason: "High risk categories must be cleaned manually".to_string(),
            });
            continue;
        }

        paths.extend(resolve_cleanable_paths(rule)?);
    }

    let mut report = clean_paths(paths, safe_mode)?;
    report.skipped_count += skipped_errors.len() as u64;
    report.errors.extend(skipped_errors);
    Ok(report)
}

#[tauri::command]
pub async fn get_clean_report(_snapshot_id: String) -> Result<CleanReport, String> {
    Err("Clean report persistence is not implemented yet".to_string())
}

fn clean_paths(paths: Vec<String>, safe_mode: bool) -> Result<CleanReport, String> {
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

fn expand_home(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(stripped);
        }
    }
    PathBuf::from(path)
}

fn expand_rule_patterns(paths: &[String]) -> Vec<String> {
    let home = dirs::home_dir().unwrap_or_default();
    paths
        .iter()
        .filter(|path| !path.contains("$("))
        .map(|path| {
            if let Some(stripped) = path.strip_prefix("~/") {
                home.join(stripped).to_string_lossy().to_string()
            } else {
                path.clone()
            }
        })
        .collect()
}

fn resolve_cleanable_paths(rule: &CategoryRule) -> Result<Vec<String>, String> {
    let mut paths = Vec::new();
    for pattern in expand_rule_patterns(&rule.paths) {
        for path in glob(&pattern).map_err(|e| e.to_string())?.filter_map(Result::ok) {
            if should_exclude(&path, &rule.exclude) {
                continue;
            }
            paths.push(path.to_string_lossy().to_string());
        }
    }
    Ok(paths)
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

fn should_exclude(path: &Path, exclude: &[String]) -> bool {
    let path_text = path.to_string_lossy();
    exclude.iter().any(|pattern| {
        let needle = pattern.trim_matches('*');
        !needle.is_empty() && path_text.contains(needle)
    })
}

fn path_size(path: &Path) -> u64 {
    if path.is_file() {
        return fs::metadata(path).map(|meta| meta.len()).unwrap_or(0);
    }

    if !path.is_dir() {
        return 0;
    }

    WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter_map(|entry| entry.metadata().ok())
        .filter(|metadata| metadata.is_file())
        .map(|metadata| metadata.len())
        .sum()
}
