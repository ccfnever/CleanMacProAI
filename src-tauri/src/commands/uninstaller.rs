/// 软件卸载 — Tauri Commands

use crate::models::InstalledApp;
use plist::Value;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

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

            if let Some(app) = read_app_bundle(&path) {
                apps.push(app);
            }
        }
    }

    apps.sort_by(|left, right| {
        (right.app_size + right.related_size).cmp(&(left.app_size + left.related_size))
    });
    Ok(apps)
}

#[tauri::command]
pub async fn uninstall_app(bundle_id: String, move_to_trash: bool) -> Result<String, String> {
    let apps = list_installed_apps().await?;
    let app = apps
        .into_iter()
        .find(|item| item.bundle_id == bundle_id)
        .ok_or_else(|| "Application not found".to_string())?;

    if app.is_system_app {
        return Err("Refusing to uninstall a system application".to_string());
    }

    if move_to_trash {
        trash::delete(&app.app_path).map_err(|e| e.to_string())?;
    } else {
        fs::remove_dir_all(&app.app_path).map_err(|e| e.to_string())?;
    }

    Ok(format!("{} removed", app.name))
}

fn read_app_bundle(path: &Path) -> Option<InstalledApp> {
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

    let app_size = path_size(path);
    let (related_size, related_count) = related_app_data(&name, &bundle_id);
    let app_path = path.to_string_lossy().to_string();
    let is_system_app = app_path.starts_with("/System/");

    Some(InstalledApp {
        name,
        bundle_id,
        app_path,
        app_size,
        related_size,
        related_count,
        is_system_app,
    })
}

fn related_app_data(name: &str, bundle_id: &str) -> (u64, u64) {
    let Some(home) = dirs::home_dir() else {
        return (0, 0);
    };

    let roots = [
        home.join("Library/Caches"),
        home.join("Library/Logs"),
        home.join("Library/Preferences"),
        home.join("Library/Application Support"),
        home.join("Library/Saved Application State"),
    ];

    let normalized_name = normalize(name);
    let normalized_bundle = normalize(bundle_id);
    let mut total_size = 0_u64;
    let mut total_count = 0_u64;

    for root in roots {
        if !root.exists() {
            continue;
        }

        let Ok(entries) = fs::read_dir(root) else {
            continue;
        };

        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            let filename = path
                .file_name()
                .and_then(|value| value.to_str())
                .map(normalize)
                .unwrap_or_default();

            if filename.contains(&normalized_bundle) || filename.contains(&normalized_name) {
                total_size += path_size(&path);
                total_count += path_count(&path);
            }
        }
    }

    (total_size, total_count)
}

fn normalize(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect()
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

fn path_count(path: &Path) -> u64 {
    if path.is_file() {
        return 1;
    }

    if !path.is_dir() {
        return 0;
    }

    WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_file())
        .count() as u64
}
