/// 清理规则引擎 — 从 YAML 加载并验证清理规则

use crate::models::RiskLevel;
use glob::Pattern;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// YAML 规则文件的结构
#[derive(Debug, Deserialize)]
pub struct CleanupRules {
    /// 规则文件版本（用于未来兼容性）
    pub version: u32,
    /// 各清理分类
    pub categories: HashMap<String, CategoryRule>,
    #[serde(default)]
    pub always_exclude: Vec<String>,
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

/// Return whether a path is covered by any exclusion pattern.
///
/// Literal patterns protect the exact path and all descendants. Glob patterns
/// use the same path-aware matching semantics while normalizing separators and
/// expanding a leading `~/` against the current user's home directory.
pub fn path_matches_any(path: &Path, patterns: &[String]) -> bool {
    let candidate = normalize_path(&path.to_string_lossy());
    patterns.iter().any(|pattern| path_matches_pattern(&candidate, pattern))
}

fn path_matches_pattern(candidate: &str, pattern: &str) -> bool {
    let is_home_pattern = pattern == "~" || pattern.starts_with("~/");
    let expanded = expand_home_pattern(pattern);
    let normalized = normalize_path(&expanded);
    if normalized.is_empty() {
        return false;
    }

    if has_glob_metachar(&normalized) {
        let Ok(compiled) = Pattern::new(&normalized) else {
            return false;
        };
        if compiled.matches_path(Path::new(candidate)) {
            return true;
        }
        // Rule-local patterns such as `com.example.App` are basename/segment
        // matches rather than filesystem roots.
        if !normalized.starts_with('/') && !is_home_pattern {
            return candidate
                .split('/')
                .any(|segment| compiled.matches(segment));
        }
        return false;
    }

    if !normalized.starts_with('/') && !is_home_pattern {
        return candidate.split('/').any(|segment| segment == normalized);
    }

    candidate == normalized
        || candidate
            .strip_prefix(&normalized)
            .is_some_and(|rest| rest.starts_with('/'))
}

fn expand_home_pattern(pattern: &str) -> String {
    if pattern == "~" {
        return dirs::home_dir()
            .map(|home| home.to_string_lossy().into_owned())
            .unwrap_or_default();
    }
    if let Some(rest) = pattern.strip_prefix("~/") {
        return dirs::home_dir()
            .map(|home| home.join(rest).to_string_lossy().into_owned())
            .unwrap_or_default();
    }
    pattern.to_string()
}

fn has_glob_metachar(value: &str) -> bool {
    value.chars().any(|ch| matches!(ch, '*' | '?' | '['))
}

fn normalize_path(value: &str) -> String {
    let value = value.replace('\\', "/");
    let absolute = value.starts_with('/');
    let mut segments: Vec<&str> = Vec::new();
    for segment in value.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop();
            }
            _ => segments.push(segment),
        }
    }
    let joined = segments.join("/");
    if absolute {
        format!("/{joined}")
    } else {
        joined
    }
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

    #[test]
    fn exclusion_matches_exact_and_descendant_but_not_similar_prefix() {
        let patterns = vec!["/Users/test/Library/Caches".to_string()];
        assert!(path_matches_any(Path::new("/Users/test/Library/Caches"), &patterns));
        assert!(path_matches_any(
            Path::new("/Users/test/Library/Caches/App/data"),
            &patterns
        ));
        assert!(!path_matches_any(
            Path::new("/Users/test/Library/Caches-old"),
            &patterns
        ));
    }

    #[test]
    fn exclusion_supports_globs_and_separator_normalization() {
        let patterns = vec!["/Users/test/Library/Caches/*".to_string()];
        assert!(path_matches_any(
            Path::new(r"\\Users\\test\\Library\\Caches\\App"),
            &patterns
        ));
        assert!(path_matches_any(
            Path::new("/Users/test/Library/Caches/App/data"),
            &patterns
        ));
        assert!(!path_matches_any(
            Path::new("/Users/test/Library/Cache/App"),
            &patterns
        ));
    }

    #[test]
    fn exclusion_expands_home_prefix() {
        let home = dirs::home_dir().expect("home directory should be available");
        let patterns = vec!["~/Library/Caches".to_string()];
        assert!(path_matches_any(&home.join("Library/Caches/App"), &patterns));
    }

    #[test]
    fn relative_exclusion_matches_a_path_segment() {
        let patterns = vec!["com.example.App".to_string()];
        assert!(path_matches_any(
            Path::new("/Users/test/Library/Caches/com.example.App/data"),
            &patterns
        ));
        assert!(!path_matches_any(
            Path::new("/Users/test/Library/Caches/com.example.App-old"),
            &patterns
        ));
    }
}
