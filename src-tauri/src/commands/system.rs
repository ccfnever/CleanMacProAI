/// 系统操作 — Tauri Commands

use crate::models::DiskInfo;
use std::path::{Path, PathBuf};
use std::process::Command;

#[tauri::command]
pub async fn get_disk_info() -> Result<DiskInfo, String> {
    let mount_path = primary_storage_mount();
    let output = Command::new("df")
        .args(["-k", mount_path])
        .output()
        .map_err(|e| format!("Failed to execute df: {}", e))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout
        .lines()
        .nth(1)
        .ok_or_else(|| "Unexpected df output".to_string())?;
    let columns: Vec<&str> = line.split_whitespace().collect();
    if columns.len() < 4 {
        return Err("Unexpected df columns".to_string());
    }

    let total_bytes = parse_kilobytes(columns[1])?;
    let available_bytes = parse_kilobytes(columns[3])?;
    let used_bytes = total_bytes.saturating_sub(available_bytes);
    let usage_percent = if total_bytes == 0 {
        0.0
    } else {
        (used_bytes as f64 / total_bytes as f64) * 100.0
    };

    Ok(DiskInfo {
        volume_name: "Macintosh HD".to_string(),
        total_bytes,
        available_bytes,
        used_bytes,
        usage_percent,
    })
}

#[tauri::command]
pub async fn request_permissions() -> Result<bool, String> {
    let _ = Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles")
        .status()
        .map_err(|e| format!("Failed to open System Settings: {}", e))?;
    Ok(true)
}

#[tauri::command]
pub async fn open_in_finder(path: String) -> Result<(), String> {
    let expanded = expand_home(&path);
    if !expanded.is_absolute() {
        return Err("Finder paths must be absolute or start with ~/".to_string());
    }
    let canonical = expanded
        .canonicalize()
        .map_err(|error| format!("Path does not exist or cannot be opened: {error}"))?;

    let mut command = Command::new("/usr/bin/open");
    if canonical.is_dir() {
        command.arg(&canonical);
    } else {
        command.arg("-R").arg(&canonical);
    }

    let status = command
        .status()
        .map_err(|error| format!("Failed to open Finder: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("Finder exited with status {status}"))
    }
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

fn parse_kilobytes(value: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map(|kb| kb.saturating_mul(1024))
        .map_err(|e| format!("Invalid df number '{}': {}", value, e))
}

fn primary_storage_mount() -> &'static str {
    if Path::new("/System/Volumes/Data").exists() {
        "/System/Volumes/Data"
    } else {
        "/"
    }
}
