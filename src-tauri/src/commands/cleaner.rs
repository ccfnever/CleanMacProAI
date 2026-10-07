/// 清理引擎 — Tauri Commands

use crate::models::{CleanError, CleanReport};
use crate::rules::{load_rules, path_matches_any, CategoryRule};
use glob::glob;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const RULES_YAML: &str = include_str!("../rules/cleanup_rules.yaml");

#[tauri::command]
pub async fn clean_items(paths: Vec<String>) -> Result<CleanReport, String> {
    tauri::async_runtime::spawn_blocking(move || clean_paths(paths))
        .await
        .map_err(|error| format!("Cleanup task failed: {error}"))?
}

#[tauri::command]
pub async fn clean_categories(
    category_ids: Vec<String>,
) -> Result<CleanReport, String> {
    tauri::async_runtime::spawn_blocking(move || clean_category_paths(category_ids))
        .await
        .map_err(|error| format!("Cleanup task failed: {error}"))?
}

fn clean_category_paths(category_ids: Vec<String>) -> Result<CleanReport, String> {
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

        paths.extend(resolve_cleanable_paths(rule, &rules.always_exclude)?);
    }

    let mut report = clean_paths(paths)?;
    report.skipped_count += skipped_errors.len() as u64;
    report.errors.extend(skipped_errors);
    Ok(report)
}

fn clean_paths(paths: Vec<String>) -> Result<CleanReport, String> {
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
        let result = delete_clean_target(&path_buf);

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
    })
}

fn delete_clean_target(path: &Path) -> Result<(), String> {
    // Inspect the entry itself: unlink symlinks without deleting their target.
    let metadata = fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    if metadata.is_dir() {
        fs::remove_dir_all(path).map_err(|error| error.to_string())
    } else {
        fs::remove_file(path).map_err(|error| error.to_string())
    }
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

fn resolve_cleanable_paths(rule: &CategoryRule, global_exclude: &[String]) -> Result<Vec<String>, String> {
    let mut paths = Vec::new();
    for pattern in expand_rule_patterns(&rule.paths) {
        for path in glob(&pattern).map_err(|e| e.to_string())?.filter_map(Result::ok) {
            if path_matches_any(&path, &rule.exclude) || path_matches_any(&path, global_exclude) {
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
    let Some(home) = dirs::home_dir() else {
        return false;
    };

    let allowed_roots = [
        home.join("Library/Caches"),
        home.join("Library/Logs"),
        home.join("Library/Developer/Xcode"),
        home.join("Downloads"),
        home.join(".Trash"),
        home.join(".npm"),
        home.join(".pnpm-store"),
        home.join(".yarn/cache"),
        home.join(".yarn/berry/cache"),
        PathBuf::from("/Library/Caches"),
        PathBuf::from("/Library/Logs"),
    ];

    allowed_roots.iter().any(|root| canonical != *root && canonical.starts_with(root))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permanently_deletes_disposable_files_and_nested_directories() {
        let root = std::env::temp_dir().join(format!("cleanmac-cleaner-{}", std::process::id()));
        fs::create_dir_all(root.join("folder/nested")).unwrap();
        fs::write(root.join("file"), b"disposable test data").unwrap();
        fs::write(root.join("folder/nested/file"), b"disposable test data").unwrap();
        delete_clean_target(&root.join("file")).unwrap();
        delete_clean_target(&root.join("folder")).unwrap();
        assert!(!root.join("file").exists());
        assert!(!root.join("folder").exists());
        assert!(delete_clean_target(&root.join("missing")).is_err());
        fs::remove_dir(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn deleting_symlink_preserves_target() {
        let root = std::env::temp_dir().join(format!("cleanmac-cleaner-link-{}", std::process::id()));
        fs::create_dir_all(root.join("target")).unwrap();
        fs::write(root.join("target/file"), b"keep").unwrap();
        std::os::unix::fs::symlink(root.join("target"), root.join("link")).unwrap();
        delete_clean_target(&root.join("link")).unwrap();
        assert!(root.join("target/file").exists());
        assert!(fs::symlink_metadata(root.join("link")).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cleanup_rejects_protected_roots_and_personal_documents() {
        let home = dirs::home_dir().unwrap();
        assert!(!is_safe_clean_target(&home));
        assert!(!is_safe_clean_target(&home.join("Library/Caches")));
        assert!(!is_safe_clean_target(&home.join("Documents")));
        assert!(!is_safe_clean_target(Path::new("/")));
    }
}
