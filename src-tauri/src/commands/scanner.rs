/// 扫描引擎 — Tauri Commands

use crate::models::{CategoryResult, FileInfo, ScanProgress, ScanResult};
use crate::rules::{load_rules, validate_rules};
use glob::glob;
use std::fs;
use std::path::Path;
use std::sync::Mutex;
use std::time::Instant;
use tauri::State;
use walkdir::WalkDir;

const MAX_PREVIEW_FILES: usize = 20;
const MAX_FILES_PER_CATEGORY: u64 = 80_000;
const RULES_YAML: &str = include_str!("../rules/cleanup_rules.yaml");

/// 全局扫描状态（简单实现，生产环境可换成 RwLock + cancellation token）
pub struct ScanState {
    pub progress: ScanProgress,
}

#[tauri::command]
pub async fn scan_system(state: State<'_, Mutex<ScanState>>) -> Result<ScanResult, String> {
    let started_at = Instant::now();
    let rules = load_rules(RULES_YAML)?;
    if rules.version != 1 {
        return Err(format!("Unsupported cleanup rules version: {}", rules.version));
    }
    let warnings = validate_rules(&rules);
    if !warnings.is_empty() {
        return Err(warnings.join("; "));
    }

    {
        let mut scan_state = state.lock().map_err(|e| e.to_string())?;
        scan_state.progress = ScanProgress {
            is_scanning: true,
            current_category: None,
            completed_categories: 0,
            total_categories: rules.categories.len() as u32,
            scanned_files: 0,
            found_bytes: 0,
        };
    }

    let mut categories = Vec::new();
    let mut total_size = 0_u64;
    let mut ordered_rules: Vec<_> = rules.categories.iter().collect();
    ordered_rules.sort_by(|left, right| left.0.cmp(right.0));

    for (category_id, rule) in ordered_rules {
        {
            let mut scan_state = state.lock().map_err(|e| e.to_string())?;
            if !scan_state.progress.is_scanning {
                break;
            }
            scan_state.progress.current_category = Some(rule.name.clone());
        }

        let mut file_count = 0_u64;
        let mut category_size = 0_u64;
        let mut preview_files = Vec::new();

        for pattern in expand_rule_patterns(&rule.paths) {
            for path in glob(&pattern).map_err(|e| e.to_string())?.filter_map(Result::ok) {
                scan_path(
                    &path,
                    &rule.exclude,
                    rule.min_size,
                    &mut file_count,
                    &mut category_size,
                    &mut preview_files,
                );

                if file_count >= MAX_FILES_PER_CATEGORY {
                    break;
                }
            }
        }

        if file_count > 0 {
            total_size += category_size;
            categories.push(CategoryResult {
                id: category_id.clone(),
                name: rule.name.clone(),
                description: rule.description.clone(),
                risk: rule.risk_level(),
                file_count,
                total_size: category_size,
                files: preview_files,
            });
        }

        let mut scan_state = state.lock().map_err(|e| e.to_string())?;
        scan_state.progress.completed_categories += 1;
        scan_state.progress.scanned_files += file_count;
        scan_state.progress.found_bytes += category_size;
    }

    let mut scan_state = state.lock().map_err(|e| e.to_string())?;
    scan_state.progress.is_scanning = false;
    scan_state.progress.current_category = None;

    Ok(ScanResult {
        total_size,
        categories,
        scan_duration_ms: started_at.elapsed().as_millis() as u64,
    })
}

#[tauri::command]
pub async fn get_scan_progress(state: State<'_, Mutex<ScanState>>) -> Result<ScanProgress, String> {
    let scan_state = state.lock().map_err(|e| e.to_string())?;
    Ok(scan_state.progress.clone())
}

#[tauri::command]
pub async fn cancel_scan(state: State<'_, Mutex<ScanState>>) -> Result<(), String> {
    let mut scan_state = state.lock().map_err(|e| e.to_string())?;
    scan_state.progress.is_scanning = false;
    Ok(())
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

fn scan_path(
    path: &Path,
    exclude: &[String],
    min_size: u64,
    file_count: &mut u64,
    category_size: &mut u64,
    preview_files: &mut Vec<FileInfo>,
) {
    if should_exclude(path, exclude) {
        return;
    }

    if path.is_file() {
        collect_file(path, min_size, file_count, category_size, preview_files);
        return;
    }

    if !path.is_dir() {
        return;
    }

    for entry in WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
    {
        if *file_count >= MAX_FILES_PER_CATEGORY {
            break;
        }

        let entry_path = entry.path();
        if entry_path.is_file() && !should_exclude(entry_path, exclude) {
            collect_file(entry_path, min_size, file_count, category_size, preview_files);
        }
    }
}

fn collect_file(
    path: &Path,
    min_size: u64,
    file_count: &mut u64,
    category_size: &mut u64,
    preview_files: &mut Vec<FileInfo>,
) {
    let Ok(metadata) = fs::metadata(path) else {
        return;
    };
    let size = metadata.len();
    if size < min_size {
        return;
    }

    *file_count += 1;
    *category_size += size;

    if preview_files.len() < MAX_PREVIEW_FILES {
        preview_files.push(FileInfo {
            path: display_path(path),
            size,
            modified_at: metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|duration| duration.as_secs().to_string()),
        });
    }
}

fn should_exclude(path: &Path, exclude: &[String]) -> bool {
    let path_text = path.to_string_lossy();
    exclude.iter().any(|pattern| {
        let needle = pattern.trim_matches('*');
        !needle.is_empty() && path_text.contains(needle)
    })
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
