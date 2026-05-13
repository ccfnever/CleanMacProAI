/// 软件卸载 — Tauri Commands

use crate::models::InstalledApp;

#[tauri::command]
pub async fn list_installed_apps() -> Result<Vec<InstalledApp>, String> {
    // TODO: 使用 NSWorkspace / LaunchServices 枚举已安装应用
    // 1. 扫描 /Applications 和 ~/Applications
    // 2. 读取 .app/Contents/Info.plist 获取 Bundle ID
    // 3. 查找 ~/Library 中的关联文件（按 Bundle ID / 应用名匹配）
    // 4. 计算总大小

    Ok(vec![])
}

#[tauri::command]
pub async fn uninstall_app(_bundle_id: String, _move_to_trash: bool) -> Result<String, String> {
    // TODO: 实现完整卸载
    // 1. 退出运行中的应用（如果正在运行）
    // 2. 删除 .app 包
    // 3. 删除关联文件（缓存/偏好设置/日志等）
    // 4. 返回卸载报告

    Err("Not implemented yet".to_string())
}
