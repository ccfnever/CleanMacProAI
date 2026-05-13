/// 系统操作 — Tauri Commands

use crate::models::DiskInfo;

#[tauri::command]
pub async fn get_disk_info() -> Result<DiskInfo, String> {
    // TODO: 获取磁盘信息
    // 使用 statfs 或 NSFileManager 获取 / 卷的容量信息

    Ok(DiskInfo {
        volume_name: "Macintosh HD".to_string(),
        total_bytes: 500_277_790_720,   // ~500 GB
        available_bytes: 100_000_000_000,
        used_bytes: 400_277_790_720,
        usage_percent: 80.0,
    })
}

#[tauri::command]
pub async fn request_permissions() -> Result<bool, String> {
    // TODO: 请求 Full Disk Access 权限
    // 调用 tauri-plugin-macos-permissions
    // 前端根据结果决定是否继续

    Ok(true)
}
