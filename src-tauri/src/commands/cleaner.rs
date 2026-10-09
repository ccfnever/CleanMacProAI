//! 清理引擎 — Tauri Commands

use super::{
    deletion,
    path_safety::{self, FileIdentity},
};
use crate::models::{CleanError, CleanReport, DeletionMode};
use crate::rules::{load_rules, path_matches_any, CategoryRule};
use glob::glob;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const RULES_YAML: &str = include_str!("../rules/cleanup_rules.yaml");

#[tauri::command]
pub async fn clean_items(
    paths: Vec<String>,
    mode: Option<DeletionMode>,
    permanent_confirmed: Option<bool>,
) -> Result<CleanReport, String> {
    let mode = deletion::confirmed_mode(mode, permanent_confirmed)?;
    tauri::async_runtime::spawn_blocking(move || clean_paths(paths, mode))
        .await
        .map_err(|error| format!("Cleanup task failed: {error}"))?
}

#[tauri::command]
pub async fn clean_categories(
    category_ids: Vec<String>,
    mode: Option<DeletionMode>,
    permanent_confirmed: Option<bool>,
) -> Result<CleanReport, String> {
    let mode = deletion::confirmed_mode(mode, permanent_confirmed)?;
    tauri::async_runtime::spawn_blocking(move || clean_category_paths(category_ids, mode))
        .await
        .map_err(|error| format!("Cleanup task failed: {error}"))?
}

fn clean_category_paths(
    category_ids: Vec<String>,
    mode: DeletionMode,
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

        paths.extend(resolve_cleanable_paths(rule, &rules.always_exclude)?);
    }

    let mut report = clean_paths(paths, mode)?;
    report.skipped_count += skipped_errors.len() as u64;
    report.errors.extend(skipped_errors);
    Ok(report)
}

fn clean_paths(paths: Vec<String>, mode: DeletionMode) -> Result<CleanReport, String> {
    let mut cleaned_count = 0_u64;
    let mut processed_bytes = 0_u64;
    let rules = load_rules(RULES_YAML)?;
    let mut seen = HashSet::new();
    let mut skipped_count = 0_u64;
    let mut errors = Vec::new();

    for path in paths {
        let path_buf = expand_home(&path);

        let validated = validate_clean_target(&path_buf, &rules, mode);
        let target = match validated {
            Ok(target) => target,
            Err(reason) => {
                skipped_count += 1;
                errors.push(CleanError { path, reason });
                continue;
            }
        };
        if !seen.insert(target.clone()) {
            continue;
        }
        let identity = match FileIdentity::capture(&target) {
            Ok(identity) => identity,
            Err(reason) => {
                skipped_count += 1;
                errors.push(CleanError { path, reason });
                continue;
            }
        };
        let size = path_size(&target);
        let result = deletion::execute(&target, &identity, mode);

        match result {
            Ok(()) => {
                cleaned_count += 1;
                processed_bytes += size;
            }
            Err(reason) => {
                skipped_count += 1;
                errors.push(CleanError { path, reason });
            }
        }
    }

    Ok(CleanReport {
        cleaned_count,
        freed_bytes: if mode == DeletionMode::Permanent {
            processed_bytes
        } else {
            0
        },
        processed_bytes,
        deletion_mode: mode,
        skipped_count,
        errors,
    })
}

fn validate_clean_target(
    path: &Path,
    rules: &crate::rules::CleanupRules,
    mode: DeletionMode,
) -> Result<PathBuf, String> {
    let target = path_safety::validate_delete(path)?;
    let home = dirs::home_dir().ok_or("无法确定用户目录")?;
    if target.starts_with(home.join(".Trash")) && mode != DeletionMode::Permanent {
        return Err("废纸篓内容只能在明确确认永久删除后清空".into());
    }
    // Patterns are bundled backend scopes; inspect each path ancestor so a
    // selected directory and its descendants use the same authorization.
    let matches_rule = |rule: &CategoryRule| {
        expand_rule_patterns(&rule.paths).iter().any(|pattern| {
            let Ok(pattern) = glob::Pattern::new(pattern) else {
                return false;
            };
            target.ancestors().any(|ancestor| {
                pattern.matches_path_with(
                    ancestor,
                    glob::MatchOptions {
                        require_literal_separator: true,
                        ..Default::default()
                    },
                )
            })
        })
    };
    if rules
        .categories
        .values()
        .any(|rule| rule.risk == "high" && matches_rule(rule))
    {
        return Err("高风险项目只能手动管理，不能通过文件路径绕过分类限制".into());
    }
    let matching: Vec<_> = rules
        .categories
        .values()
        .filter(|rule| matches!(rule.risk.as_str(), "low" | "medium") && matches_rule(rule))
        .collect();
    if matching.is_empty() {
        return Err("路径不在清理规则允许范围内".into());
    }
    // Use the union of applicable exclusions, independent of HashMap order.
    path_safety::validate_tree(&target, |entry| {
        path_matches_any(entry, &rules.always_exclude)
            || matching
                .iter()
                .any(|rule| path_matches_any(entry, &rule.exclude))
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

fn resolve_cleanable_paths(
    rule: &CategoryRule,
    global_exclude: &[String],
) -> Result<Vec<String>, String> {
    let mut paths = Vec::new();
    for pattern in expand_rule_patterns(&rule.paths) {
        for path in glob(&pattern)
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
        {
            if path_matches_any(&path, &rule.exclude) || path_matches_any(&path, global_exclude) {
                continue;
            }
            paths.push(path.to_string_lossy().to_string());
        }
    }
    Ok(paths)
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
    fn rejects_archives_high_risk_and_non_rule_paths_in_both_modes() {
        let rules = load_rules(RULES_YAML).unwrap();
        let home = dirs::home_dir().unwrap();
        for mode in [DeletionMode::Trash, DeletionMode::Permanent] {
            for path in [
                home.clone(),
                home.join("Documents"),
                home.join("Library/Developer/Xcode/Archives"),
                PathBuf::from("/"),
            ] {
                assert!(validate_clean_target(&path, &rules, mode).is_err());
            }
        }
        let report =
            clean_category_paths(vec!["xcode_archives".into()], DeletionMode::Trash).unwrap();
        assert_eq!(report.cleaned_count, 0);
        assert_eq!(report.skipped_count, 1);
        assert_eq!(report.freed_bytes, 0);
        assert_eq!(report.deletion_mode, DeletionMode::Trash);
    }
    #[test]
    fn arbitrary_paths_and_descendant_exclusions_cannot_bypass_rules() {
        let root = std::env::temp_dir().join(format!("cleanmac-clean-rule-{}", std::process::id()));
        fs::create_dir_all(root.join("cache")).unwrap();
        fs::write(root.join("cache/keep.plist"), "keep").unwrap();
        fs::write(root.join("cache/disposable"), "delete").unwrap();
        fs::write(root.join("outside"), "keep").unwrap();
        let root = root.canonicalize().unwrap();
        let yaml = format!("version: 1\ncategories:\n  fixture:\n    name: fixture\n    description: fixture\n    risk: low\n    paths: ['{}/cache']\n    exclude: ['*.plist']\n", root.display());
        let rules = load_rules(&yaml).unwrap();
        assert!(
            validate_clean_target(&root.join("outside"), &rules, DeletionMode::Permanent).is_err()
        );
        assert!(
            validate_clean_target(&root.join("cache"), &rules, DeletionMode::Permanent).is_err()
        );
        assert!(validate_clean_target(
            &root.join("cache/keep.plist"),
            &rules,
            DeletionMode::Permanent
        )
        .is_err());
        assert!(
            validate_clean_target(&root.join("cache/disposable"), &rules, DeletionMode::Trash)
                .is_ok()
        );
        let high_yaml = format!("version: 1\ncategories:\n  fixture:\n    name: fixture\n    description: fixture\n    risk: high\n    paths: ['{}/cache/*']\n", root.display());
        let high_rules = load_rules(&high_yaml).unwrap();
        assert!(validate_clean_target(
            &root.join("cache/disposable"),
            &high_rules,
            DeletionMode::Permanent
        )
        .is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
