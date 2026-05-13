/// 清理引擎 — Tauri Commands

use crate::models::CleanReport;

#[tauri::command]
pub async fn clean_items(
    _paths: Vec<String>,
    _safe_mode: bool, // true = 移到废纸篓, false = 直接删除（需确认）
) -> Result<CleanReport, String> {
    // TODO: 实现清理逻辑
    // 1. 创建清理快照（记录原路径，用于回滚）
    // 2. 根据 safe_mode 决定删除方式
    // 3. 逐个处理文件，记录成功/失败
    // 4. 返回清理报告

    // MVP 阶段：模拟返回
    Ok(CleanReport {
        cleaned_count: 0,
        freed_bytes: 0,
        skipped_count: 0,
        errors: vec![],
        snapshot_id: "snapshot-001".to_string(),
    })
}

#[tauri::command]
pub async fn get_clean_report(_snapshot_id: String) -> Result<CleanReport, String> {
    // TODO: 从快照存储加载报告
    Err("Not implemented yet".to_string())
}
