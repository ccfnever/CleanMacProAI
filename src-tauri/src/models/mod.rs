/// 数据模型定义

use serde::{Deserialize, Serialize};

// ── 扫描结果 ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    /// 总可清理大小（字节）
    pub total_size: u64,
    /// 各分类清理项
    pub categories: Vec<CategoryResult>,
    /// 扫描耗时（毫秒）
    pub scan_duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryResult {
    /// 分类 ID（对应 rules 中的 key）
    pub id: String,
    /// 分类名称
    pub name: String,
    /// 分类描述
    pub description: String,
    /// 风险等级
    pub risk: RiskLevel,
    /// 找到的文件数
    pub file_count: u64,
    /// 总大小（字节）
    pub total_size: u64,
    /// 具体文件列表（预览，最多 20 条）
    pub files: Vec<FileInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    /// 文件路径
    pub path: String,
    /// 文件大小（字节）
    pub size: u64,
    /// 最后修改时间
    pub modified_at: Option<String>,
}

// ── 风险等级 ──

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    /// 安全 — 可以放心删除
    Low,
    /// 中等 — 建议确认后删除
    Medium,
    /// 高风险 — 默认不勾选
    High,
}

// ── 清理执行结果 ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanReport {
    /// 成功删除的文件数
    pub cleaned_count: u64,
    /// 释放的空间（字节）
    pub freed_bytes: u64,
    /// 跳过的文件数
    pub skipped_count: u64,
    /// 错误列表
    pub errors: Vec<CleanError>,
    /// 快照 ID（用于回滚）
    pub snapshot_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanError {
    pub path: String,
    pub reason: String,
}

// ── 已安装应用 ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledApp {
    /// 应用名称
    pub name: String,
    /// Bundle ID
    pub bundle_id: String,
    /// 应用路径
    pub app_path: String,
    /// 应用大小（字节）
    pub app_size: u64,
    /// 关联文件总大小（缓存、日志、偏好设置等）
    pub related_size: u64,
    /// 关联文件数量
    pub related_count: u64,
    /// 关联文件预览（缓存、日志、偏好设置等顶层命中项）
    pub related_files: Vec<FileInfo>,
    /// 是否系统应用
    pub is_system_app: bool,
}

// ── 磁盘信息 ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo {
    /// 卷名
    pub volume_name: String,
    /// 总容量（字节）
    pub total_bytes: u64,
    /// 可用空间（字节）
    pub available_bytes: u64,
    /// 已用空间（字节）
    pub used_bytes: u64,
    /// 使用百分比
    pub usage_percent: f64,
}

// ── 扫描进度 ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanProgress {
    /// 是否正在扫描
    pub is_scanning: bool,
    /// 当前扫描的分类
    pub current_category: Option<String>,
    /// 已扫描分类数
    pub completed_categories: u32,
    /// 总分类数
    pub total_categories: u32,
    /// 已扫描文件数
    pub scanned_files: u64,
    /// 已发现可清理大小
    pub found_bytes: u64,
}
