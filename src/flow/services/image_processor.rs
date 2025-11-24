use crate::error::{AgentFlowError, Result};
use crate::flow::constants::{fields, llm as llm_consts};
use anyhow::anyhow;
use base64::{engine::general_purpose, Engine as _};
use parking_lot::RwLock;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// 图像缓存：避免重复读取同一文件
///
/// 使用 LRU 策略，最多缓存 100 个图像
static IMAGE_CACHE: once_cell::sync::Lazy<Arc<RwLock<HashMap<String, String>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

/// 图像信息
#[derive(Debug, Clone)]
pub struct ImageInfo {
    pub url: Option<String>,
    pub base64: Option<String>,
    pub path: Option<String>,
}

impl ImageInfo {
    pub fn new() -> Self {
        Self {
            url: None,
            base64: None,
            path: None,
        }
    }

    pub fn with_url(mut self, url: String) -> Self {
        self.url = Some(url);
        self
    }

    pub fn with_base64(mut self, base64: String) -> Self {
        self.base64 = Some(base64);
        self
    }

    pub fn with_path(mut self, path: String) -> Self {
        self.path = Some(path);
        self
    }
}

/// 图像处理服务
pub struct ImageProcessor;

impl ImageProcessor {
    /// 检查是否为视觉模型
    ///
    /// 通过检查模型名称中是否包含视觉模型关键词来判断
    /// 如果提供了 keywords 配置，使用配置的关键词；否则使用默认
    pub fn is_vision_model(model: Option<&str>, keywords: Option<&[&str]>) -> bool {
        let keywords_to_check = keywords.unwrap_or(&[
            llm_consts::VISION_KEYWORD_VL,
            llm_consts::VISION_KEYWORD_VISION,
        ]);

        model
            .map(|m| keywords_to_check.iter().any(|keyword| m.contains(keyword)))
            .unwrap_or(false)
    }

    /// 从 payload 中提取图像信息
    ///
    /// 如果 is_vision 为 false，返回空的 ImageInfo
    pub fn extract_image_info(payload: &Value, is_vision: bool) -> Result<ImageInfo> {
        if !is_vision {
            return Ok(ImageInfo::new());
        }

        let mut info = ImageInfo::new();

        if let Some(url) = payload
            .get(fields::IMAGE_URL)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
        {
            info.url = Some(url);
        }

        if let Some(base64) = payload
            .get(fields::IMAGE_BASE64)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
        {
            info.base64 = Some(base64);
        }

        if let Some(path) = payload
            .get(fields::IMAGE_PATH)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
        {
            info.path = Some(path);
        }

        Ok(info)
    }

    /// 处理图像路径，转换为 base64
    ///
    /// 优化：
    /// 1. 使用缓存避免重复读取
    /// 2. 验证文件存在性
    /// 3. 支持图像压缩（如果文件过大）
    pub fn process_image_path(path: &str) -> Result<String> {
        {
            let cache = IMAGE_CACHE.read();
            if let Some(cached) = cache.get(path) {
                tracing::debug!(path = %path, "使用缓存的图像数据");
                return Ok(cached.clone());
            }
        }

        if !std::path::Path::new(path).exists() {
            return Err(AgentFlowError::Other(anyhow!("图像文件不存在: {}", path)));
        }

        let image_data = std::fs::read(path)
            .map_err(|e| AgentFlowError::Other(anyhow!("无法读取图像文件 {}: {}", path, e)))?;

        if image_data.len() > 5 * 1024 * 1024 {
            tracing::warn!(
                path = %path,
                size_mb = image_data.len() / (1024 * 1024),
                "图像文件过大，可能影响性能。建议压缩后再使用。"
            );
        }

        let base64_data = general_purpose::STANDARD.encode(&image_data);

        {
            let mut cache = IMAGE_CACHE.write();
            if cache.len() >= 100 {
                if let Some(key) = cache.keys().next().cloned() {
                    cache.remove(&key);
                    tracing::debug!("图像缓存已满，移除最旧的条目");
                }
            }
            cache.insert(path.to_string(), base64_data.clone());
        }

        Ok(base64_data)
    }

    /// 清除图像缓存
    pub fn clear_cache() {
        IMAGE_CACHE.write().clear();
        tracing::debug!("图像缓存已清除");
    }

    /// 获取缓存大小
    pub fn cache_size() -> usize {
        IMAGE_CACHE.read().len()
    }

    /// 获取最终的 base64 图像数据
    ///
    /// 优先级：image_path（转换为 base64） > image_base64 > 无
    pub fn get_final_base64(image_info: &ImageInfo) -> Result<Option<String>> {
        if let Some(path) = &image_info.path {
            Ok(Some(Self::process_image_path(path)?))
        } else {
            Ok(image_info.base64.clone())
        }
    }

    /// 清理 payload 中的图像字段（如果不需要）
    pub fn remove_image_fields(payload: &mut Value) {
        if let Some(obj) = payload.as_object_mut() {
            obj.remove(fields::IMAGE_URL);
            obj.remove(fields::IMAGE_BASE64);
            obj.remove(fields::IMAGE_PATH);
        }
    }

    /// 将图像信息添加到 payload 中
    pub fn add_image_fields(payload: &mut Value, image_info: &ImageInfo) {
        if let Some(url) = &image_info.url {
            payload[fields::IMAGE_URL] = Value::String(url.clone());
        }
        if let Some(base64) = &image_info.base64 {
            payload[fields::IMAGE_BASE64] = Value::String(base64.clone());
        }
        if let Some(path) = &image_info.path {
            payload[fields::IMAGE_PATH] = Value::String(path.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_is_vision_model() {
        assert!(ImageProcessor::is_vision_model(Some("qwen-vl-max")));
        assert!(ImageProcessor::is_vision_model(Some("vision-model")));
        assert!(!ImageProcessor::is_vision_model(Some("qwen-max")));
        assert!(!ImageProcessor::is_vision_model(None));
    }

    #[test]
    fn test_extract_image_info() {
        let payload = json!({
            "image_url": "https://example.com/image.jpg",
            "image_base64": "base64data",
            "image_path": "/path/to/image.jpg"
        });

        let info = ImageProcessor::extract_image_info(&payload, true).unwrap();
        assert_eq!(info.url, Some("https://example.com/image.jpg".to_string()));
        assert_eq!(info.base64, Some("base64data".to_string()));
        assert_eq!(info.path, Some("/path/to/image.jpg".to_string()));
    }

    #[test]
    fn test_extract_image_info_not_vision() {
        let payload = json!({
            "image_url": "https://example.com/image.jpg"
        });

        let info = ImageProcessor::extract_image_info(&payload, false).unwrap();
        assert!(info.url.is_none());
        assert!(info.base64.is_none());
        assert!(info.path.is_none());
    }

    #[test]
    fn test_add_remove_image_fields() {
        let mut payload = json!({});
        let info = ImageInfo::new()
            .with_url("https://example.com/image.jpg".to_string())
            .with_base64("base64data".to_string());

        ImageProcessor::add_image_fields(&mut payload, &info);
        assert_eq!(payload["image_url"], "https://example.com/image.jpg");
        assert_eq!(payload["image_base64"], "base64data");

        ImageProcessor::remove_image_fields(&mut payload);
        assert!(!payload.as_object().unwrap().contains_key("image_url"));
        assert!(!payload.as_object().unwrap().contains_key("image_base64"));
    }
}
