/// 服务层辅助函数
///
/// 提供通用的辅助功能，减少代码重复
use crate::error::{AgentFlowError, Result};
use anyhow::anyhow;
use serde_json::Value;

/// 字符串处理辅助
pub struct StringHelper;

impl StringHelper {
    /// 安全地截断字符串用于日志显示
    ///
    /// 如果字符串过长，截断并添加省略号
    pub fn truncate_for_log(s: &str, max_len: usize) -> String {
        if s.len() <= max_len {
            s.to_string()
        } else {
            format!("{}...(已截断，总长度: {} 字符)", &s[..max_len], s.len())
        }
    }

    /// 屏蔽 API Key 用于日志显示
    ///
    /// 只显示前4位和后4位，中间用 *** 代替
    pub fn mask_api_key(key: &str) -> String {
        if key.len() > 8 {
            format!("{}***{}", &key[..4], &key[key.len() - 4..])
        } else {
            "***".to_string()
        }
    }

    /// 清理 JSON 响应（移除 Markdown 代码块标记）
    pub fn clean_json_response(response: &str) -> String {
        let trimmed = response.trim();

        if trimmed.starts_with("```json") && trimmed.ends_with("```") {
            let content = &trimmed[7..trimmed.len() - 3];
            content.trim().to_string()
        } else if trimmed.starts_with("```") && trimmed.ends_with("```") {
            let content = &trimmed[3..trimmed.len() - 3];
            content.trim().to_string()
        } else {
            trimmed.to_string()
        }
    }
}

/// JSON 处理辅助
pub struct JsonHelper;

impl JsonHelper {
    /// 安全地获取 JSON 字段（字符串类型）
    pub fn get_string(obj: &Value, key: &str) -> Option<String> {
        obj.get(key)?.as_str().map(|s| s.to_string())
    }

    /// 安全地获取 JSON 字段（数字类型）
    pub fn get_number(obj: &Value, key: &str) -> Option<f64> {
        obj.get(key)?.as_f64()
    }

    /// 安全地获取 JSON 字段（布尔类型）
    pub fn get_bool(obj: &Value, key: &str) -> Option<bool> {
        obj.get(key)?.as_bool()
    }

    /// 安全地获取 JSON 数组
    pub fn get_array<'a>(obj: &'a Value, key: &str) -> Option<&'a Vec<Value>> {
        obj.get(key)?.as_array()
    }

    /// 安全地获取嵌套字段
    ///
    /// 例如：get_nested(&obj, &["user", "profile", "name"])
    pub fn get_nested<'a>(obj: &'a Value, path: &[&str]) -> Option<&'a Value> {
        let mut current = obj;
        for key in path {
            current = current.get(key)?;
        }
        Some(current)
    }

    /// 合并两个 JSON 对象
    ///
    /// target 中的字段会被 source 中的同名字段覆盖
    pub fn merge(target: &mut Value, source: &Value) {
        if let (Some(target_obj), Some(source_obj)) = (target.as_object_mut(), source.as_object()) {
            for (key, value) in source_obj {
                target_obj.insert(key.clone(), value.clone());
            }
        }
    }
}

/// 时间处理辅助
pub struct TimeHelper;

impl TimeHelper {
    /// 获取当前 ISO8601 时间戳
    pub fn now_iso8601() -> String {
        use std::time::SystemTime;
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();

        format!("{}.{:03}Z", now.as_secs(), now.subsec_millis())
    }

    /// 获取当前 Unix 时间戳（秒）
    pub fn now_unix() -> u64 {
        use std::time::SystemTime;
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }
}

/// 文件处理辅助
pub struct FileHelper;

impl FileHelper {
    /// 安全地读取文件为字符串
    pub fn read_to_string(path: &str) -> Result<String> {
        std::fs::read_to_string(path)
            .map_err(|e| AgentFlowError::Other(anyhow!("无法读取文件 {}: {}", path, e)))
    }

    /// 安全地读取文件为字节
    pub fn read_to_bytes(path: &str) -> Result<Vec<u8>> {
        std::fs::read(path)
            .map_err(|e| AgentFlowError::Other(anyhow!("无法读取文件 {}: {}", path, e)))
    }

    /// 检查文件是否存在
    pub fn exists(path: &str) -> bool {
        std::path::Path::new(path).exists()
    }

    /// 获取文件大小（字节）
    pub fn file_size(path: &str) -> Result<u64> {
        let metadata = std::fs::metadata(path)
            .map_err(|e| AgentFlowError::Other(anyhow!("无法获取文件信息 {}: {}", path, e)))?;
        Ok(metadata.len())
    }

    /// 格式化文件大小（人类可读）
    pub fn format_size(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if bytes >= GB {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.2} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} B", bytes)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_truncate_for_log() {
        let short = "hello";
        assert_eq!(StringHelper::truncate_for_log(short, 10), "hello");

        let long = "hello world this is a long string";
        let truncated = StringHelper::truncate_for_log(long, 10);
        assert!(truncated.starts_with("hello worl"));
        assert!(truncated.contains("已截断"));
    }

    #[test]
    fn test_mask_api_key() {
        assert_eq!(StringHelper::mask_api_key("sk-1234567890"), "sk-1***7890");
        assert_eq!(StringHelper::mask_api_key("short"), "***");
    }

    #[test]
    fn test_clean_json_response() {
        let json_wrapped = "```json\n{\"key\": \"value\"}\n```";
        let cleaned = StringHelper::clean_json_response(json_wrapped);
        assert_eq!(cleaned, "{\"key\": \"value\"}");

        let plain = "{\"key\": \"value\"}";
        let cleaned2 = StringHelper::clean_json_response(plain);
        assert_eq!(cleaned2, plain);
    }

    #[test]
    fn test_json_helper() {
        let obj = json!({
            "name": "test",
            "age": 25,
            "active": true,
            "tags": ["a", "b"],
            "nested": {
                "field": "value"
            }
        });

        assert_eq!(
            JsonHelper::get_string(&obj, "name"),
            Some("test".to_string())
        );
        assert_eq!(JsonHelper::get_number(&obj, "age"), Some(25.0));
        assert_eq!(JsonHelper::get_bool(&obj, "active"), Some(true));
        assert_eq!(JsonHelper::get_array(&obj, "tags").unwrap().len(), 2);

        let nested = JsonHelper::get_nested(&obj, &["nested", "field"]);
        assert_eq!(nested.and_then(|v| v.as_str()), Some("value"));
    }

    #[test]
    fn test_json_merge() {
        let mut target = json!({"a": 1, "b": 2});
        let source = json!({"b": 3, "c": 4});

        JsonHelper::merge(&mut target, &source);

        assert_eq!(target["a"], 1);
        assert_eq!(target["b"], 3); // 被覆盖
        assert_eq!(target["c"], 4); // 新增
    }

    #[test]
    fn test_format_size() {
        assert_eq!(FileHelper::format_size(500), "500 B");
        assert_eq!(FileHelper::format_size(1024), "1.00 KB");
        assert_eq!(FileHelper::format_size(1024 * 1024), "1.00 MB");
        assert_eq!(FileHelper::format_size(1024 * 1024 * 1024), "1.00 GB");
    }
}
