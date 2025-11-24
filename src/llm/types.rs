use futures::Stream;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::pin::Pin;

use crate::error::Result;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LlmMessage {
    pub role: String,
    pub content: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LlmRequest {
    #[serde(default)]
    pub system: Option<String>,
    pub user: String,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default)]
    pub metadata: Option<Value>,
    #[serde(default)]
    pub image_url: Option<String>,
    #[serde(default)]
    pub image_base64: Option<String>,
}

fn default_temperature() -> f32 {
    0.2
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LlmResponse {
    pub content: String,
    #[serde(default)]
    pub metadata: Option<Value>,
}

#[derive(Clone, Debug)]
pub struct LlmStreamChunk {
    pub content: String,
    pub done: bool,
}

pub type LlmStream = Pin<Box<dyn Stream<Item = Result<LlmStreamChunk>> + Send>>;

/// API 请求/响应格式
/// 
/// 此枚举定义了不同LLM提供商的API格式。
/// 可以从配置中指定，或者从endpoint URL自动推断。
/// 
/// # 配置示例
/// 
/// ```json
/// {
///   "driver": "custom",
///   "endpoint": "https://api.example.com/v1/chat/completions",
///   "api_format": "openai"  // 可选，不指定则自动推断
/// }
/// ```
/// 
/// # 自动推断规则
/// 
/// - endpoint包含 "/compatible-mode/" 或 "/chat/completions" → OpenAI
/// - endpoint包含 "/services/aigc/text-generation/" → Qwen
/// - 同时使用Qwen且model包含"vl" → QwenVision
/// - 其他情况 → 需要在配置中明确指定
#[cfg(feature = "openai-client")]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApiFormat {
    /// OpenAI兼容格式（最常用）
    /// 
    /// 支持的提供商：
    /// - OpenAI
    /// - 通义千问（compatible-mode）
    /// - 月之暗面（Moonshot）
    /// - 智谱AI（BigModel）
    /// - DeepSeek
    /// - OpenRouter
    /// - 豆包（Doubao）
    /// - Claude
    /// - Gemini
    /// - Mistral
    /// - 零一万物
    /// - 大部分其他兼容OpenAI API的服务
    OpenAI,
    
    /// 通义千问原生格式
    /// 
    /// 使用场景：
    /// - 不使用compatible-mode时的通义千问API
    /// - endpoint包含 "/services/aigc/"
    Qwen,
    
    /// 通义千问视觉模型格式
    /// 
    /// 使用场景：
    /// - 通义千问的视觉模型（qwen-vl系列）
    /// - 需要处理图片输入的场景
    QwenVision,
}

#[cfg(feature = "openai-client")]
impl ApiFormat {
    /// 从endpoint URL推断API格式
    /// 
    /// # 参数
    /// 
    /// - `endpoint`: API端点URL
    /// - `model`: 模型名称（可选，用于视觉模型检测）
    /// 
    /// # 返回
    /// 
    /// - `Some(ApiFormat)`: 成功推断
    /// - `None`: 无法推断，需要手动指定
    pub fn infer_from_endpoint(endpoint: &str, model: Option<&str>) -> Option<Self> {
        if endpoint.contains("compatible-mode") 
            || endpoint.contains("/chat/completions")
            || endpoint.contains("openai.com")
            || endpoint.contains("moonshot.cn")
            || endpoint.contains("bigmodel.cn")
            || endpoint.contains("deepseek.com")
            || endpoint.contains("openrouter.ai")
            || endpoint.contains("anthropic.com")
            || endpoint.contains("mistral.ai")
        {
            return Some(ApiFormat::OpenAI);
        }
        
        if endpoint.contains("dashscope.aliyuncs.com") 
            && endpoint.contains("/services/aigc/text-generation/")
        {
            if let Some(model_name) = model {
                if model_name.contains("vl") || model_name.contains("vision") {
                    return Some(ApiFormat::QwenVision);
                }
            }
            return Some(ApiFormat::Qwen);
        }
        
        None
    }
    
    /// 从字符串解析
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "openai" => Some(ApiFormat::OpenAI),
            "qwen" => Some(ApiFormat::Qwen),
            "qwenvision" | "qwen_vision" => Some(ApiFormat::QwenVision),
            _ => None,
        }
    }
    
    /// 转为字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            ApiFormat::OpenAI => "openai",
            ApiFormat::Qwen => "qwen",
            ApiFormat::QwenVision => "qwenvision",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    #[cfg(feature = "openai-client")]
    fn test_infer_openai_format() {
        assert_eq!(
            ApiFormat::infer_from_endpoint("https://api.openai.com/v1/chat/completions", None),
            Some(ApiFormat::OpenAI)
        );
        assert_eq!(
            ApiFormat::infer_from_endpoint("https://dashscope.aliyuncs.com/compatible-mode/v1", None),
            Some(ApiFormat::OpenAI)
        );
        assert_eq!(
            ApiFormat::infer_from_endpoint("https://api.moonshot.cn/v1/chat/completions", None),
            Some(ApiFormat::OpenAI)
        );
    }
    
    #[test]
    #[cfg(feature = "openai-client")]
    fn test_infer_qwen_format() {
        assert_eq!(
            ApiFormat::infer_from_endpoint(
                "https://dashscope.aliyuncs.com/api/v1/services/aigc/text-generation/generation",
                Some("qwen-max")
            ),
            Some(ApiFormat::Qwen)
        );
        assert_eq!(
            ApiFormat::infer_from_endpoint(
                "https://dashscope.aliyuncs.com/api/v1/services/aigc/text-generation/generation",
                Some("qwen-vl-max")
            ),
            Some(ApiFormat::QwenVision)
        );
    }
}
