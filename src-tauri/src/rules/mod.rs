/// 清理规则引擎 — 从 YAML 加载并验证清理规则

use crate::models::RiskLevel;
use serde::Deserialize;
use std::collections::HashMap;

/// YAML 规则文件的结构
#[derive(Debug, Deserialize)]
pub struct CleanupRules {
    /// 规则文件版本（用于未来兼容性）
    pub version: u32,
    /// 各清理分类
    pub categories: HashMap<String, CategoryRule>,
}

#[derive(Debug, Deserialize)]
pub struct CategoryRule {
    /// 显示名称
    pub name: String,
    /// 描述说明
    pub description: String,
    /// 风险等级
    pub risk: String,
    /// 要扫描的路径模式列表
    pub paths: Vec<String>,
    /// 排除模式（不扫描的路径）
    #[serde(default)]
    pub exclude: Vec<String>,
    /// 文件大小限制（字节），小于此值不统计
    #[serde(default)]
    pub min_size: u64,
}

impl CategoryRule {
    /// 将字符串 risk 转为 RiskLevel 枚举
    pub fn risk_level(&self) -> RiskLevel {
        match self.risk.as_str() {
            "low" => RiskLevel::Low,
            "medium" => RiskLevel::Medium,
            "high" => RiskLevel::High,
            _ => RiskLevel::Medium, // 默认中等风险
        }
    }

}

/// 加载并解析规则文件
pub fn load_rules(yaml_content: &str) -> Result<CleanupRules, String> {
    serde_yaml::from_str(yaml_content).map_err(|e| format!("Failed to parse rules: {}", e))
}

/// 验证规则的合法性（安全检查）
pub fn validate_rules(rules: &CleanupRules) -> Vec<String> {
    let mut warnings = vec![];

    for (key, rule) in &rules.categories {
        // 警告：包含高风险路径的规则
        for path in &rule.paths {
            if path.contains("/System/") || path.contains("/usr/bin/") {
                warnings.push(format!(
                    "Category '{}' contains sensitive system path: {}",
                    key, path
                ));
            }
        }
    }

    warnings
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RULES: &str = r#"
version: 1
categories:
  system_cache:
    name: "系统缓存"
    description: "应用和系统缓存文件"
    risk: low
    paths:
      - "~/Library/Caches/*"
      - "/Library/Caches/*"
    exclude:
      - "com.apple.Safari"
  xcode_derived:
    name: "Xcode 构建缓存"
    description: "Xcode 编译中间产物"
    risk: low
    paths:
      - "~/Library/Developer/Xcode/DerivedData/*"
"#;

    #[test]
    fn test_load_rules() {
        let rules = load_rules(SAMPLE_RULES).expect("Should parse valid YAML");
        assert_eq!(rules.version, 1);
        assert!(rules.categories.contains_key("system_cache"));
        assert!(rules.categories.contains_key("xcode_derived"));
    }

    #[test]
    fn test_risk_level() {
        let rules = load_rules(SAMPLE_RULES).unwrap();
        let cache_rule = &rules.categories["system_cache"];
        assert_eq!(cache_rule.risk_level(), RiskLevel::Low);
    }

    #[test]
    fn test_validate_rules_warns_sensitive_paths() {
        let sensitive_rules = r#"
version: 1
categories:
  dangerous:
    name: "危险操作"
    description: "测试"
    risk: high
    paths:
      - "/System/Library/Caches/*"
"#;
        let rules = load_rules(sensitive_rules).unwrap();
        let warnings = validate_rules(&rules);
        assert!(!warnings.is_empty());
        assert!(warnings[0].contains("sensitive system path"));
    }
}
