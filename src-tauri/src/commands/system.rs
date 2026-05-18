/// 系统操作 — Tauri Commands

use crate::models::DiskInfo;
use std::process::Command;

#[tauri::command]
pub async fn get_disk_info() -> Result<DiskInfo, String> {
    let output = Command::new("df")
        .args(["-k", "/"])
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
    let used_bytes = parse_kilobytes(columns[2])?;
    let available_bytes = parse_kilobytes(columns[3])?;
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
    // Tauri 插件已注册；后续可以接入 Full Disk Access 引导 UI。
    Ok(true)
}

fn parse_kilobytes(value: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map(|kb| kb.saturating_mul(1024))
        .map_err(|e| format!("Invalid df number '{}': {}", value, e))
}
