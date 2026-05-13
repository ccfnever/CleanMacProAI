/// 扫描引擎 — Tauri Commands

use crate::models::{ScanResult, ScanProgress};
use std::sync::Mutex;
use tauri::State;

/// 全局扫描状态（简单实现，生产环境用 RwLock）
pub struct ScanState {
    pub progress: ScanProgress,
}

// ── Tauri Commands ──

#[tauri::command]
pub async fn scan_system(state: State<'_, Mutex<ScanState>>) -> Result<ScanResult, String> {
    // TODO: 实现完整扫描逻辑
    // 1. 加载清理规则
    // 2. 按分类异步扫描
    // 3. 实时更新进度
    // 4. 返回汇总结果

    let mut scan_state = state.lock().map_err(|e| e.to_string())?;
    scan_state.progress.is_scanning = true;

    // MVP 阶段：硬编码测试数据，后续替换为真实扫描
    let result = ScanResult {
        total_size: 2_147_483_648, // ~2 GB
        categories: vec![],
        scan_duration_ms: 3_500,
    };

    scan_state.progress.is_scanning = false;
    Ok(result)
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
