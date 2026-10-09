//! Third-party launchd registrations. Never edit/delete plists or stop running services.
use plist::Value;
use serde::Serialize;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Clone, Serialize)]
pub struct StartupItem {
    id: String,
    name: String,
    label: String,
    kind: String,
    path: String,
    program: String,
    arguments: Vec<String>,
    enabled: Option<bool>,
    missing_target: bool,
    manageable: bool,
    requires_authorization: bool,
    management_note: String,
}

#[derive(Serialize)]
pub struct StartupScan {
    items: Vec<StartupItem>,
    warnings: Vec<String>,
}

fn user_domain() -> Result<String, String> {
    let output = Command::new("/usr/bin/id")
        .arg("-u")
        .output()
        .map_err(|e| e.to_string())?;
    let uid = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !output.status.success() || uid.parse::<u32>().is_err() {
        return Err("无法获取当前用户".into());
    }
    Ok(format!("gui/{uid}"))
}

fn disabled_overrides(text: &str) -> HashMap<String, bool> {
    text.lines()
        .filter_map(|line| {
            let (label, value) = line.trim().split_once(" => ")?;
            let label = label.strip_prefix('"')?.strip_suffix('"')?;
            let disabled = match value.trim().trim_end_matches([',', ';']) {
                "true" | "disabled" => true,
                "false" | "enabled" => false,
                _ => return None,
            };
            Some((label.to_string(), disabled))
        })
        .collect()
}

fn read_overrides(domain: &str) -> Result<HashMap<String, bool>, String> {
    let output = Command::new("/bin/launchctl")
        .args(["print-disabled", domain])
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "无法读取 {domain} 启动状态：{}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(disabled_overrides(&String::from_utf8_lossy(&output.stdout)))
}

fn read_item(
    path: &Path,
    kind: &str,
    overrides: Option<&HashMap<String, bool>>,
) -> Result<StartupItem, String> {
    let value = Value::from_file(path).map_err(|e| format!("{}：{e}", path.display()))?;
    let dict = value.as_dictionary().ok_or("启动配置不是字典")?;
    let label = dict
        .get("Label")
        .and_then(Value::as_string)
        .filter(|s| !s.is_empty())
        .ok_or("启动配置缺少 Label")?
        .to_string();
    let arguments: Vec<String> = dict
        .get("ProgramArguments")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_string)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let program = dict
        .get("Program")
        .and_then(Value::as_string)
        .map(str::to_string)
        .or_else(|| arguments.first().cloned())
        .unwrap_or_default();
    // Relative programs can resolve through launchd's PATH; do not label them as missing.
    let missing_target = Path::new(&program).is_absolute() && !Path::new(&program).exists();
    let name = program
        .split_once(".app/")
        .and_then(|(prefix, _)| Path::new(prefix).file_name())
        .and_then(|n| n.to_str())
        .map(str::to_string)
        .unwrap_or_else(|| label.clone());
    let default_disabled = dict
        .get("Disabled")
        .and_then(Value::as_boolean)
        .unwrap_or(false);
    let enabled = overrides.map(|states| !states.get(&label).copied().unwrap_or(default_disabled));
    let manageable = enabled.is_some()
        && !label.starts_with("com.apple.")
        && !label.contains('/')
        && !label.chars().any(char::is_control)
        && std::fs::symlink_metadata(path)
            .map(|m| m.is_file() && !m.file_type().is_symlink())
            .unwrap_or(false);
    Ok(StartupItem {
        id: path.to_string_lossy().into_owned(),
        name,
        label: label.clone(),
        kind: kind.into(),
        path: path.to_string_lossy().into_owned(),
        program,
        arguments,
        enabled,
        missing_target,
        manageable,
        requires_authorization: kind == "system_daemon",
        management_note: if enabled.is_none() {
            "无法读取启动状态，请重新扫描".into()
        } else if label.starts_with("com.apple.") {
            "此 Apple 服务受保护，请在系统设置中管理".into()
        } else if !manageable {
            "配置路径或启动标识符异常，无法直接切换".into()
        } else if kind == "system_daemon" {
            "影响所有用户，切换时需要管理员授权".into()
        } else if kind == "shared_agent" {
            "仅调整当前用户的自动启动许可".into()
        } else {
            "调整当前用户的自动启动许可".into()
        },
    })
}

fn scan() -> Result<StartupScan, String> {
    if !cfg!(target_os = "macos") {
        return Err("启动项管理目前仅支持 macOS".into());
    }
    let home = dirs::home_dir().ok_or("无法获取用户目录")?;
    let domain = user_domain()?;
    let mut warnings = Vec::new();
    let user_states = read_overrides(&domain).map_err(|e| warnings.push(e)).ok();
    let system_states = read_overrides("system").map_err(|e| warnings.push(e)).ok();
    let directories = [
        (
            home.join("Library/LaunchAgents"),
            "user_agent",
            user_states.as_ref(),
        ),
        (
            PathBuf::from("/Library/LaunchAgents"),
            "shared_agent",
            user_states.as_ref(),
        ),
        (
            PathBuf::from("/Library/LaunchDaemons"),
            "system_daemon",
            system_states.as_ref(),
        ),
    ];
    let mut items = Vec::new();
    for (directory, kind, states) in directories {
        let entries = match std::fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => {
                warnings.push(format!("无法读取 {}：{e}", directory.display()));
                continue;
            }
        };
        for entry in entries {
            let path = match entry {
                Ok(e) => e.path(),
                Err(e) => {
                    warnings.push(e.to_string());
                    continue;
                }
            };
            if path.extension().and_then(|s| s.to_str()) != Some("plist") {
                continue;
            }
            match read_item(&path, kind, states) {
                Ok(item) => items.push(item),
                Err(e) => warnings.push(format!("未能解析 {}：{e}", path.display())),
            }
        }
    }
    // A launchd label is the actual operation target. Duplicate configurations must not
    // expose independent switches that would misrepresent one shared state.
    let mut counts = HashMap::new();
    for item in &items {
        let domain = if item.kind == "system_daemon" {
            "system"
        } else {
            "gui"
        };
        *counts.entry((domain, item.label.clone())).or_insert(0usize) += 1;
    }
    for item in &mut items {
        let domain = if item.kind == "system_daemon" {
            "system"
        } else {
            "gui"
        };
        if counts
            .get(&(domain, item.label.clone()))
            .copied()
            .unwrap_or(0)
            > 1
        {
            item.manageable = false;
            item.management_note = "同一启动标识符存在多份配置，请在所属应用中检查".into();
            warnings.push(format!(
                "{} 存在重复的启动标识符，请在所属应用中检查配置",
                item.label
            ));
        }
    }
    items.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then(a.id.cmp(&b.id))
    });
    Ok(StartupScan { items, warnings })
}

#[tauri::command]
pub async fn list_startup_items() -> Result<StartupScan, String> {
    tokio::task::spawn_blocking(scan)
        .await
        .map_err(|e| e.to_string())?
}

fn validate_config_path(path: &Path, directory: &Path) -> Result<(), String> {
    let metadata = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    let canonical_directory = directory.canonicalize().map_err(|e| e.to_string())?;
    let canonical_path = path.canonicalize().map_err(|e| e.to_string())?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || canonical_path.parent() != Some(canonical_directory.as_path())
        || canonical_path.extension().and_then(|s| s.to_str()) != Some("plist")
    {
        return Err("只能管理指定启动目录中的普通 plist 文件".into());
    }
    Ok(())
}

// Infer privilege scope from the validated path, never from a client-provided flag.
fn config_scope(path: &Path, home: &Path) -> Result<(PathBuf, &'static str), String> {
    for (directory, kind) in [
        (home.join("Library/LaunchAgents"), "user_agent"),
        (PathBuf::from("/Library/LaunchAgents"), "shared_agent"),
        (PathBuf::from("/Library/LaunchDaemons"), "system_daemon"),
    ] {
        if path.parent() == Some(directory.as_path()) {
            return Ok((directory, kind));
        }
    }
    Err("启动配置不在允许管理的目录内".into())
}

// Two different languages interpret this string: quote for POSIX shell first,
// then for AppleScript. Only a fixed launchctl operation can run as administrator.
fn authorization_script(target: &str, enabled: bool) -> String {
    let quoted_target = format!("'{}'", target.replace('\'', "'\"'\"'"));
    let command = format!(
        "/bin/launchctl {} {quoted_target}",
        if enabled { "enable" } else { "disable" }
    );
    let escaped = command
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r");
    format!("do shell script \"{escaped}\" with administrator privileges")
}

fn update_launch_permission(
    target: &str,
    enabled: bool,
    administrator: bool,
) -> Result<(), String> {
    let output = if administrator {
        Command::new("/usr/bin/osascript")
            .args(["-e", &authorization_script(target, enabled)])
            .output()
    } else {
        Command::new("/bin/launchctl")
            .args([if enabled { "enable" } else { "disable" }, target])
            .output()
    }
    .map_err(|e| format!("无法执行启动项操作：{e}"))?;
    if output.status.success() {
        return Ok(());
    }
    let error = String::from_utf8_lossy(&output.stderr);
    if administrator && error.contains("(-128)") {
        return Err("已取消管理员授权，启动项状态未更改".into());
    }
    Err(format!("无法更新启动项：{}", error.trim()))
}

fn set_enabled(path: String, enabled: bool) -> Result<StartupItem, String> {
    if !cfg!(target_os = "macos") {
        return Err("启动项管理目前仅支持 macOS".into());
    }
    let path = PathBuf::from(path);
    let home = dirs::home_dir().ok_or("无法获取用户目录")?;
    let (directory, kind) = config_scope(&path, &home)?;
    validate_config_path(&path, &directory)?;
    let domain = if kind == "system_daemon" {
        "system".to_string()
    } else {
        user_domain()?
    };
    let states = read_overrides(&domain)?;
    let item = read_item(&path, kind, Some(&states))?;
    if !item.manageable {
        return Err(item.management_note);
    }
    let same_domain =
        |other: &&StartupItem| (other.kind == "system_daemon") == (kind == "system_daemon");
    if scan()?
        .items
        .iter()
        .filter(same_domain)
        .filter(|other| other.label == item.label)
        .count()
        != 1
    {
        return Err("启动标识符存在重复或配置已变化，请重新扫描并在所属应用中检查".into());
    }
    let target = format!("{domain}/{}", item.label);
    update_launch_permission(&target, enabled, item.requires_authorization)?;
    let states = read_overrides(&domain)?;
    let updated = read_item(&path, kind, Some(&states))?;
    if updated.enabled != Some(enabled) {
        return Err("系统未确认状态变更，请重新扫描".into());
    }
    Ok(updated)
}

#[tauri::command]
pub async fn set_startup_item_enabled(path: String, enabled: bool) -> Result<StartupItem, String> {
    tokio::task::spawn_blocking(move || set_enabled(path, enabled))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn open_startup_settings() -> Result<(), String> {
    let status = Command::new("/usr/bin/open")
        .arg("x-apple.systempreferences:com.apple.LoginItems-Settings.extension")
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("无法打开登录项设置，请在系统设置 → 通用 → 登录项中查看".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_explicit_enable_and_disable_overrides() {
        let states = disabled_overrides("disabled services = {\n\t\"com.example.off\" => true\n\t\"com.example.on\" => false\n}");
        assert_eq!(states.get("com.example.off"), Some(&true));
        assert_eq!(states.get("com.example.on"), Some(&false));
        assert_eq!(states.len(), 2);
        let current =
            disabled_overrides("\"com.example.off\" => disabled\n\"com.example.on\" => enabled");
        assert_eq!(current, states);
    }
    #[test]
    #[cfg(target_os = "macos")]
    fn native_scan_is_read_only_and_validates_item_scopes() {
        let result = scan().unwrap();
        assert!(result
            .items
            .iter()
            .all(|item| !item.manageable || item.enabled.is_some()));
    }
    #[test]
    fn scope_is_derived_from_the_configuration_directory() {
        let home = Path::new("/Users/test");
        assert_eq!(
            config_scope(
                Path::new("/Users/test/Library/LaunchAgents/test.plist"),
                home
            )
            .unwrap()
            .1,
            "user_agent"
        );
        assert_eq!(
            config_scope(Path::new("/Library/LaunchAgents/test.plist"), home)
                .unwrap()
                .1,
            "shared_agent"
        );
        assert_eq!(
            config_scope(Path::new("/Library/LaunchDaemons/test.plist"), home)
                .unwrap()
                .1,
            "system_daemon"
        );
        assert!(config_scope(Path::new("/System/Library/LaunchDaemons/test.plist"), home).is_err());
        assert!(config_scope(Path::new("/tmp/test.plist"), home).is_err());
    }
    #[test]
    fn administrator_script_quotes_both_interpreters() {
        assert_eq!(authorization_script("system/com.example.test", false), "do shell script \"/bin/launchctl disable 'system/com.example.test'\" with administrator privileges");
        let script = authorization_script("system/test'$(touch /tmp/should-not-exist)\\\"", true);
        assert!(script.contains("'\\\"'\\\"'"));
        assert!(script.contains("\\\\\\\""));
        assert!(script.starts_with("do shell script \"/bin/launchctl enable 'system/"));
    }
    #[test]
    fn plist_state_respects_overrides_and_unknown_status() {
        let directory =
            std::env::temp_dir().join(format!("cleanmac-startup-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("test.plist");
        let mut dict = plist::Dictionary::new();
        dict.insert("Label".into(), Value::String("com.example.test".into()));
        dict.insert("Disabled".into(), Value::Boolean(true));
        dict.insert(
            "ProgramArguments".into(),
            Value::Array(vec![Value::String(
                "/Applications/Absent.app/Contents/MacOS/Absent".into(),
            )]),
        );
        Value::Dictionary(dict).to_file_xml(&path).unwrap();
        let mut states = HashMap::new();
        assert_eq!(
            read_item(&path, "user_agent", Some(&states))
                .unwrap()
                .enabled,
            Some(false)
        );
        states.insert("com.example.test".into(), false);
        let item = read_item(&path, "user_agent", Some(&states)).unwrap();
        assert_eq!(item.enabled, Some(true));
        assert_eq!(item.name, "Absent");
        assert!(item.missing_target && item.manageable);
        let system = read_item(&path, "system_daemon", Some(&states)).unwrap();
        assert!(system.manageable && system.requires_authorization);
        let shared = read_item(&path, "shared_agent", Some(&states)).unwrap();
        assert!(shared.manageable && !shared.requires_authorization);
        assert_eq!(read_item(&path, "user_agent", None).unwrap().enabled, None);
        assert!(validate_config_path(&path, &directory).is_ok());
        assert!(validate_config_path(&path, &directory.join("elsewhere")).is_err());
        #[cfg(unix)]
        {
            let link = directory.join("link.plist");
            std::os::unix::fs::symlink(&path, &link).unwrap();
            assert!(validate_config_path(&link, &directory).is_err());
        }
        std::fs::remove_dir_all(directory).unwrap();
    }
}
