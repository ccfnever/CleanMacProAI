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
    /// 分类根路径下的一级文件夹/文件明细，按大小降序排列
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
    /// 是否为文件夹
    pub is_dir: bool,
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
    /// 应用图标路径（用于调试/兜底）
    pub icon_path: Option<String>,
    /// 应用图标 data URL，前端可直接作为 img src 使用
    pub icon_data_url: Option<String>,
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

// ── 空间地图 ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceMapResult {
    /// 本次分析的规范化根路径
    pub root_path: String,
    /// 用于界面展示的路径（用户目录会缩写为 ~）
    pub display_path: String,
    /// 已读取文件的逻辑大小，仅供内部汇总兼容
    pub logical_size: u64,
    /// 文件系统报告的已分配块大小（APFS 克隆/压缩仍可能共享物理块）
    pub allocated_size: u64,
    /// 成功统计的唯一普通文件数
    pub file_count: u64,
    /// 成功遍历的目录数（不含分析根目录）
    pub directory_count: u64,
    /// 本次检查过的普通文件总数（包含被阈值忽略的小文件）
    pub scanned_file_count: u64,
    /// 小于阈值、未进入空间地图的文件数
    pub ignored_file_count: u64,
    /// 被阈值忽略的小文件逻辑大小合计
    pub ignored_logical_size: u64,
    /// 纳入结果的最小磁盘已分配空间
    pub minimum_file_size: u64,
    /// 根目录的直接子项，包含其递归汇总值
    pub entries: Vec<SpaceMapEntry>,
    /// 因权限、并发删除或 I/O 错误无法读取的条目数
    pub skipped_items: u64,
    /// 已跳过的重复硬链接数，避免重复计算占用
    pub hard_link_duplicates: u64,
    /// 未跟随的符号链接数，避免越界和循环
    pub symlink_count: u64,
    /// 分析耗时（毫秒）
    pub scan_duration_ms: u64,
    /// 用户停止后的部分结果
    pub incomplete: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SpaceMapProgress {
    pub is_scanning: bool,
    pub root_path: String,
    pub display_path: String,
    pub current_path: Option<String>,
    pub minimum_file_size: u64,
    pub scanned_file_count: u64,
    pub scanned_directory_count: u64,
    pub matched_file_count: u64,
    pub matched_logical_size: u64,
    pub matched_allocated_size: u64,
    pub ignored_file_count: u64,
    pub ignored_logical_size: u64,
    pub skipped_items: u64,
    pub hard_link_duplicates: u64,
    pub symlink_count: u64,
    pub entries: Vec<SpaceMapEntry>,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceMapEntry {
    pub name: String,
    pub path: String,
    pub logical_size: u64,
    pub allocated_size: u64,
    pub file_count: u64,
    pub directory_count: u64,
    pub modified_at: Option<String>,
    pub is_dir: bool,
    /// macOS 应用、照片图库等包目录，进入前需要用户明确选择
    pub is_package: bool,
    /// iCloud 尚未完整下载到本机的占位条目
    pub is_cloud_placeholder: bool,
    #[serde(default)]
    pub children: Vec<SpaceMapEntry>,
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
