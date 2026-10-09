//! 软件卸载 — Tauri Commands

use super::{
    deletion,
    path_safety::{self, FileIdentity},
};
use crate::models::{CleanError, CleanReport, DeletionMode, FileInfo, InstalledApp};
use plist::Value;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

#[tauri::command]
pub async fn list_installed_apps() -> Result<Vec<InstalledApp>, String> {
    tokio::task::spawn_blocking(list_installed_apps_blocking)
        .await
        .map_err(|error| format!("Application scan task failed: {error}"))?
}

fn list_installed_apps_blocking() -> Result<Vec<InstalledApp>, String> {
    list_apps_from_roots(application_roots())
}

fn list_apps_from_roots(roots: Vec<PathBuf>) -> Result<Vec<InstalledApp>, String> {
    let mut apps = Vec::new();
    let mut seen_paths = HashSet::new();
    for root in roots {
        if !root.exists() {
            continue;
        }

        let Ok(entries) = fs::read_dir(root) else {
            continue;
        };

        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if !is_visible_app_bundle(&path) {
                continue;
            }

            let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
            if !seen_paths.insert(canonical) {
                continue;
            }
            if let Some(app) = read_app_bundle(&path, false) {
                apps.push(app);
            }
        }
    }

    apps.sort_by_key(|app| app.name.to_lowercase());
    Ok(apps)
}

fn is_visible_app_bundle(path: &Path) -> bool {
    path.extension().and_then(|ext| ext.to_str()) == Some("app")
        && path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| !name.starts_with('.'))
}

#[tauri::command]
pub async fn inspect_installed_app(
    bundle_id: String,
    app_path: Option<String>,
) -> Result<InstalledApp, String> {
    tokio::task::spawn_blocking(move || inspect_installed_app_blocking(bundle_id, app_path))
        .await
        .map_err(|error| format!("Application inspection task failed: {error}"))?
}

fn inspect_installed_app_blocking(
    bundle_id: String,
    app_path: Option<String>,
) -> Result<InstalledApp, String> {
    if let Some(app_path) = app_path {
        let path = expand_home(&app_path);
        validate_application_path(&path)?;
        let identity = FileIdentity::capture(&path)?;
        if let Some(app) = read_app_bundle(&path, true) {
            if app.bundle_id == bundle_id {
                identity.verify(&path)?;
                return Ok(app);
            }
        }
    }

    find_app_by_bundle_id(&bundle_id, true)
        .ok_or_else(|| format!("Application '{}' not found", bundle_id))
}

#[tauri::command]
pub async fn uninstall_app(
    bundle_id: String,
    app_path: Option<String>,
    mode: Option<DeletionMode>,
    permanent_confirmed: Option<bool>,
) -> Result<CleanReport, String> {
    let mode = deletion::confirmed_mode(mode, permanent_confirmed)?;
    tokio::task::spawn_blocking(move || uninstall_app_blocking(bundle_id, app_path, mode))
        .await
        .map_err(|error| format!("Application uninstall task failed: {error}"))?
}

fn uninstall_app_blocking(
    bundle_id: String,
    app_path: Option<String>,
    mode: DeletionMode,
) -> Result<CleanReport, String> {
    let (app, app_identity) = resolve_app_for_uninstall(&bundle_id, app_path)?;

    if app.is_system_app {
        return Err("Refusing to uninstall a system application".to_string());
    }

    let mut targets = vec![app.app_path.clone()];
    targets.extend(app.related_files.iter().map(|file| file.path.clone()));
    let mut seen = HashSet::new();
    targets.retain(|target| seen.insert(target.clone()));

    let mut cleaned_count = 0_u64;
    let mut processed_bytes = 0_u64;
    let mut skipped_count = 0_u64;
    let mut errors = Vec::new();

    for (index, target) in targets.into_iter().enumerate() {
        let path = expand_home(&target);
        let validated = validate_uninstall_target(&path).and_then(|path| {
            if index == 0 {
                validate_application_path(&path)?;
            }
            path_safety::validate_tree(&path, |_| false)
        });
        let path = match validated {
            Ok(path) => path,
            Err(reason) => {
                skipped_count += 1;
                errors.push(CleanError {
                    path: target,
                    reason,
                });
                if index == 0 {
                    break;
                }
                continue;
            }
        };
        let identity = match if index == 0 {
            app_identity.verify(&path).map(|()| app_identity.clone())
        } else {
            FileIdentity::capture(&path)
        } {
            Ok(identity) => identity,
            Err(reason) => {
                skipped_count += 1;
                errors.push(CleanError {
                    path: target,
                    reason,
                });
                if index == 0 {
                    break;
                }
                continue;
            }
        };
        let size = path_size(&path);
        let count = path_count_for_report(&path);
        let result = deletion::execute(&path, &identity, mode);

        match result {
            Ok(()) => {
                cleaned_count += count;
                processed_bytes += size;
            }
            Err(reason) => {
                skipped_count += 1;
                errors.push(CleanError {
                    path: target,
                    reason,
                });
                if index == 0 {
                    break;
                }
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

fn resolve_app_for_uninstall(
    bundle_id: &str,
    app_path: Option<String>,
) -> Result<(InstalledApp, FileIdentity), String> {
    if let Some(app_path) = app_path {
        let path = expand_home(&app_path);
        validate_application_path(&path)?;
        let identity = FileIdentity::capture(&path)?;

        let app = read_app_bundle(&path, true)
            .ok_or_else(|| format!("Application not found at '{}'", app_path))?;
        if app.bundle_id != bundle_id {
            return Err(
                "Application bundle identifier does not match the selected app".to_string(),
            );
        }
        identity.verify(&path)?;
        return Ok((app, identity));
    }

    let app = find_app_by_bundle_id(bundle_id, true)
        .ok_or_else(|| "Application not found".to_string())?;
    let path = validate_application_path(&expand_home(&app.app_path))?;
    let identity = FileIdentity::capture(&path)?;
    let current = read_app_bundle(&path, true).ok_or("应用已不存在，请重新扫描")?;
    if current.bundle_id != bundle_id {
        return Err("应用身份已改变，请重新扫描".into());
    }
    identity.verify(&path)?;
    Ok((current, identity))
}

fn find_app_by_bundle_id(bundle_id: &str, include_details: bool) -> Option<InstalledApp> {
    application_roots()
        .into_iter()
        .filter(|root| root.exists())
        .filter_map(|root| fs::read_dir(root).ok())
        .flat_map(|entries| entries.filter_map(Result::ok))
        .map(|entry| entry.path())
        .filter(|path| is_visible_app_bundle(path))
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
    path_safety::existing_path(path).ok()?;
    let info_plist = path.join("Contents/Info.plist");
    path_safety::existing_path(&info_plist).ok()?;
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
    // The initial list must not launch Spotlight or image-conversion processes.
    // These are enriched one app at a time after the list is already visible.
    let icon = if include_details {
        app_icon_asset(path, &bundle_id, dictionary)
    } else {
        None
    };
    let is_system_app = app_path.starts_with("/System/")
        || bundle_id.starts_with("com.apple.")
        || bundle_id == "com.cleanmacproai.desktop";

    Some(InstalledApp {
        name,
        bundle_id,
        app_path,
        last_opened_at: if include_details {
            app_last_opened_at(path)
        } else {
            None
        },
        icon_path: icon.as_ref().map(|asset| asset.path.clone()),
        icon_data_url: icon.and_then(|asset| asset.data_url),
        app_size,
        related_size: related.total_size,
        related_count: related.total_count,
        related_files: related.preview_files,
        is_system_app,
    })
}

fn app_last_opened_at(path: &Path) -> Option<i64> {
    let output = command_output_with_timeout(
        Command::new("/usr/bin/mdls")
            .args(["-raw", "-name", "kMDItemLastUsedDate"])
            .arg(path),
        Duration::from_millis(500),
    )?;
    if !output.status.success() {
        return None;
    }
    parse_last_opened_at(&String::from_utf8(output.stdout).ok()?)
}

fn command_output_with_timeout(command: &mut Command, timeout: Duration) -> Option<Output> {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return child.wait_with_output().ok(),
            Ok(None) if started.elapsed() < timeout => {
                std::thread::sleep(Duration::from_millis(10));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
}

fn parse_last_opened_at(value: &str) -> Option<i64> {
    chrono::DateTime::parse_from_str(value.trim().trim_matches('"'), "%Y-%m-%d %H:%M:%S %z")
        .ok()
        .map(|date| date.timestamp())
}

struct IconAsset {
    path: String,
    data_url: Option<String>,
}

fn app_icon_asset(
    bundle_path: &Path,
    bundle_id: &str,
    dictionary: &plist::Dictionary,
) -> Option<IconAsset> {
    let icon_name = dictionary
        .get("CFBundleIconFile")
        .and_then(Value::as_string)
        .or_else(|| {
            dictionary
                .get("CFBundleIcons")
                .and_then(Value::as_dictionary)
                .and_then(|icons| icons.get("CFBundlePrimaryIcon"))
                .and_then(Value::as_dictionary)
                .and_then(|primary| primary.get("CFBundleIconFiles"))
                .and_then(Value::as_array)
                .and_then(|files| files.iter().filter_map(Value::as_string).next_back())
        })?;

    let resources = bundle_path.join("Contents/Resources");
    let candidates = if Path::new(icon_name).extension().is_some() {
        vec![resources.join(icon_name)]
    } else {
        vec![
            resources.join(format!("{icon_name}.icns")),
            resources.join(icon_name),
        ]
    };

    let source = candidates.into_iter().find(|path| path.exists())?;
    let extension = source
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");

    let display_path = if extension.eq_ignore_ascii_case("icns") {
        convert_icns_to_png(&source, bundle_id).unwrap_or_else(|| source.clone())
    } else {
        source
    };

    Some(IconAsset {
        data_url: image_data_url(&display_path),
        path: display_path.to_string_lossy().to_string(),
    })
}

fn convert_icns_to_png(source: &Path, bundle_id: &str) -> Option<PathBuf> {
    let cache_root = dirs::cache_dir()
        .or_else(dirs::data_local_dir)?
        .join("CleanMacProAI")
        .join("app-icons");

    fs::create_dir_all(&cache_root).ok()?;

    let file_name = format!("{}.png", safe_file_stem(bundle_id));
    let output_path = cache_root.join(file_name);

    if output_path.exists() {
        return Some(output_path);
    }

    let output = command_output_with_timeout(
        Command::new("sips")
            .args(["-s", "format", "png"])
            .arg(source)
            .arg("--out")
            .arg(&output_path),
        Duration::from_secs(2),
    )?;

    if output.status.success() && output_path.exists() {
        Some(output_path)
    } else {
        None
    }
}

fn image_data_url(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    let mime = match path.extension().and_then(|ext| ext.to_str()) {
        Some(ext) if ext.eq_ignore_ascii_case("png") => "image/png",
        Some(ext) if ext.eq_ignore_ascii_case("jpg") || ext.eq_ignore_ascii_case("jpeg") => {
            "image/jpeg"
        }
        Some(ext) if ext.eq_ignore_ascii_case("gif") => "image/gif",
        _ => return None,
    };

    Some(format!("data:{mime};base64,{}", base64_encode(&bytes)))
}

fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(bytes.len().div_ceil(3) * 4);

    for chunk in bytes.chunks(3) {
        let first = chunk[0];
        let second = *chunk.get(1).unwrap_or(&0);
        let third = *chunk.get(2).unwrap_or(&0);
        let triple = ((first as u32) << 16) | ((second as u32) << 8) | third as u32;

        output.push(TABLE[((triple >> 18) & 0x3f) as usize] as char);
        output.push(TABLE[((triple >> 12) & 0x3f) as usize] as char);
        if chunk.len() > 1 {
            output.push(TABLE[((triple >> 6) & 0x3f) as usize] as char);
        } else {
            output.push('=');
        }
        if chunk.len() > 2 {
            output.push(TABLE[(triple & 0x3f) as usize] as char);
        } else {
            output.push('=');
        }
    }

    output
}

fn safe_file_stem(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect()
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

    let candidate_paths = related_library_matches(&home, name, bundle_id);

    let mut total_size = 0_u64;
    let mut total_count = 0_u64;
    let mut preview_files = Vec::new();

    for path in candidate_paths {
        if path_safety::existing_path(&path).is_err() || !path_safety::readable(&path) {
            continue;
        }

        let size = path_size(&path);
        let count = path_count_for_report(&path);
        total_size += size;
        total_count += count;

        preview_files.push(FileInfo {
            path: display_path(&path),
            size,
            modified_at: fs::metadata(&path)
                .ok()
                .and_then(|metadata| metadata.modified().ok())
                .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|duration| duration.as_secs().to_string()),
            is_dir: path.is_dir(),
        });
    }

    RelatedAppData {
        total_size,
        total_count,
        preview_files,
    }
}

fn related_library_matches(home: &Path, name: &str, bundle_id: &str) -> Vec<PathBuf> {
    let roots = [
        home.join("Library/Caches"),
        home.join("Library/Logs"),
        home.join("Library/Preferences"),
        home.join("Library/Application Support"),
        home.join("Library/Containers"),
        home.join("Library/Group Containers"),
        home.join("Library/Saved Application State"),
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
            if is_related_filename(file_name, name, bundle_id) && seen.insert(path.clone()) {
                paths.push(path);
            }
        }
    }

    paths
}

fn is_related_filename(file_name: &str, name: &str, bundle_id: &str) -> bool {
    let normalized_file = file_name.trim().trim_end_matches(".plist").to_lowercase();
    let normalized_name = normalize_match_text(name);
    let normalized_bundle_id = bundle_id.trim().to_lowercase();

    (!normalized_name.is_empty() && normalize_match_text(&normalized_file) == normalized_name)
        || normalized_file == normalized_bundle_id
        || normalized_file.starts_with(&format!("{}.", normalized_bundle_id))
        || normalized_file.ends_with(&format!(".{}", normalized_bundle_id))
        || normalized_file.contains(&format!(".{}.", normalized_bundle_id))
}

fn normalize_match_text(value: &str) -> String {
    value
        .trim()
        .trim_end_matches(".plist")
        .chars()
        .filter(|ch| ch.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn path_size(path: &Path) -> u64 {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return 0;
    };
    if metadata.is_symlink() {
        return 0;
    }
    if metadata.is_file() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            return metadata.blocks().saturating_mul(512);
        }
        #[cfg(not(unix))]
        return metadata.len();
    }
    if metadata.is_dir() {
        du_size(path).unwrap_or(0)
    } else {
        0
    }
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
    u64::from(path.exists())
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

fn validate_application_path(path: &Path) -> Result<PathBuf, String> {
    let target = path_safety::validate_delete(path)?;
    if target.extension().and_then(|value| value.to_str()) != Some("app")
        || !application_roots()
            .iter()
            .any(|root| target.parent() == Some(root.as_path()))
    {
        return Err("只能卸载应用目录中的顶层 .app 程序".into());
    }
    Ok(target)
}

fn validate_uninstall_target(path: &Path) -> Result<PathBuf, String> {
    let target = path_safety::validate_delete(path)?;
    let home = dirs::home_dir().ok_or("无法确定用户目录")?;
    let mut roots = application_roots();
    roots.extend(
        [
            "Library/Caches",
            "Library/Logs",
            "Library/Preferences",
            "Library/Application Support",
            "Library/Containers",
            "Library/Saved Application State",
        ]
        .into_iter()
        .map(|name| home.join(name)),
    );
    for root in roots {
        if target.parent() == Some(root.as_path()) {
            return path_safety::validate_child(&root, &target);
        }
    }
    Err("路径不在后端应用及关联数据范围内".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uninstall_rejects_non_app_paths_shared_containers_and_system_apps() {
        let home = dirs::home_dir().unwrap();
        assert!(validate_application_path(&home.join("Library/Caches")).is_err());
        assert!(validate_uninstall_target(&home.join("Library/Group Containers")).is_err());
        assert!(validate_uninstall_target(Path::new("/System")).is_err());
        assert!(
            resolve_app_for_uninstall("fake.id", Some(home.to_string_lossy().into_owned()))
                .is_err()
        );
    }

    #[test]
    fn quick_list_returns_before_detail_enrichment() {
        let started = Instant::now();
        let apps = list_installed_apps_blocking().unwrap();
        println!(
            "Quick application list: {} apps in {:?}",
            apps.len(),
            started.elapsed()
        );
        assert!(apps.iter().all(|app| app.icon_data_url.is_none()
            && app.last_opened_at.is_none()
            && app.app_size == 0
            && app.related_size == 0));
    }

    #[cfg(unix)]
    #[test]
    fn metadata_command_timeout_does_not_block_detail_queue() {
        assert!(command_output_with_timeout(
            Command::new("/bin/sleep").arg("2"),
            Duration::from_millis(30)
        )
        .is_none());
        let output = command_output_with_timeout(
            Command::new("/usr/bin/printf").arg("ok"),
            Duration::from_secs(1),
        )
        .unwrap();
        assert_eq!(output.stdout, b"ok");
    }

    #[test]
    fn parses_spotlight_last_opened_date_without_inventing_missing_dates() {
        assert_eq!(
            parse_last_opened_at("2026-10-07 11:48:06 +0000"),
            parse_last_opened_at("2026-10-07 19:48:06 +0800")
        );
        assert!(parse_last_opened_at("2026-10-07 11:48:06 +0000").is_some());
        assert_eq!(parse_last_opened_at("(null)"), None);
        assert_eq!(parse_last_opened_at("not a date"), None);
    }

    #[test]
    fn lists_visible_copies_by_path_without_hidden_backups() {
        let root = std::env::temp_dir().join(format!("cleanmac-app-list-{}", std::process::id()));
        for name in ["Archive.app", "Archive copy.app", ".Archive-backup.app"] {
            let contents = root.join(name).join("Contents");
            fs::create_dir_all(&contents).unwrap();
            fs::write(contents.join("Info.plist"), r#"<?xml version="1.0"?><plist version="1.0"><dict><key>CFBundleName</key><string>Archive</string><key>CFBundleIdentifier</key><string>test.archive</string></dict></plist>"#).unwrap();
        }
        let apps = list_apps_from_roots(vec![root.clone()]).unwrap();
        assert_eq!(apps.len(), 2);
        assert_ne!(apps[0].app_path, apps[1].app_path);
        assert!(apps.iter().all(|app| app.bundle_id == "test.archive"));
        assert!(apps
            .iter()
            .all(|app| !app.app_path.contains(".Archive-backup")));
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn standalone_files_use_allocated_size_and_do_not_follow_symlinks() {
        use std::os::unix::fs::MetadataExt;
        let root = std::env::temp_dir().join(format!("cleanmac-app-size-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let file = fs::File::create(root.join("sparse")).unwrap();
        file.set_len(1024 * 1024 * 1024).unwrap();
        let metadata = file.metadata().unwrap();
        assert_eq!(path_size(&root.join("sparse")), metadata.blocks() * 512);
        assert!(path_size(&root.join("sparse")) < metadata.len());
        std::os::unix::fs::symlink(root.join("sparse"), root.join("link")).unwrap();
        assert_eq!(path_size(&root.join("link")), 0);
        fs::remove_dir_all(root).unwrap();
    }

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

    #[test]
    fn related_matching_does_not_match_similar_names() {
        assert!(is_related_filename(
            "com.tencent.xin.plist",
            "微信",
            "com.tencent.xin"
        ));
        assert!(is_related_filename(
            "com.tencent.xin.login.plist",
            "微信",
            "com.tencent.xin"
        ));
        assert!(is_related_filename(
            "5A4RE8SF68.com.tencent.xin.IPCHelper",
            "微信",
            "com.tencent.xin"
        ));
        assert!(!is_related_filename("企业微信", "微信", "com.tencent.xin"));
        assert!(!is_related_filename("微盘", "微信", "com.tencent.xin"));
    }

    #[test]
    fn related_matching_accepts_exact_app_name_only() {
        assert!(is_related_filename("WeChat", "WeChat", "com.tencent.xin"));
        assert!(!is_related_filename(
            "WeChat Helper",
            "WeChat",
            "com.tencent.xin"
        ));
    }
}
