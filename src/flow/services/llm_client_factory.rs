use std::sync::Arc;
use crate::error::{Result, AgentFlowError};
use crate::llm::DynLlmClient;
use crate::flow::config::AgentConfig;
use super::image_processor::ImageProcessor;
#[cfg(feature = "openai-client")]
use crate::GenericHttpClient;
use crate::flow::config::AgentDriverKind;
#[cfg(feature = "openai-client")]
use crate::llm::ApiFormat;
use anyhow::anyhow;

/// LLM 客户端工厂
/// 
/// 根据 AgentConfig 创建 LLM 客户端，所有参数从配置读取
/// - endpoint: 优先从 AgentConfig.endpoint，否则从 driver.default_endpoint()
/// - api_key: 优先从 AgentConfig.api_key，否则从环境变量读取
/// - model: 从 AgentConfig.model 读取
/// - format: 从 driver.api_format() 获取
/// - auth_header: 从 AgentConfig.metadata 读取（如果支持）
pub struct LlmClientFactory;

#[cfg(feature = "openai-client")]
impl LlmClientFactory {
    /// 创建 LLM 客户端
    /// 
    /// 统一使用 GenericHttpClient，所有参数从 AgentConfig 和 driver.rs 读取
    pub fn create_client(profile: &AgentConfig) -> Result<Option<DynLlmClient>> {
        match profile.driver {
            AgentDriverKind::Echo => Ok(None),
            _ => {
                let api_key = Self::get_api_key(profile)?;
                let model = profile
                    .model
                    .clone()
                    .ok_or_else(|| AgentFlowError::Other(anyhow!("Missing model configuration")))?;
                
                // 确定 endpoint：优先配置，否则使用 driver 默认值
                let endpoint = profile
                    .endpoint
                    .clone()
                    .or_else(|| profile.driver.default_endpoint().map(|s| s.to_string()))
                    .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!(
                        "Missing endpoint for driver '{}'. Please provide 'endpoint' field in agent config, or use a driver that has a default endpoint.",
                        profile.driver.as_str()
                    )))?;
                
                // 确定 API 格式：从 driver 获取
                let format = Self::determine_api_format(profile, &endpoint, &model)?;
                
                // 从 metadata 读取 auth_header（如果存在）
                let auth_header = profile
                    .metadata
                    .as_ref()
                    .and_then(|m| m.get("auth_header"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                
                // 创建 GenericHttpClient
                let client = if let Some(auth_header) = auth_header {
                    GenericHttpClient::with_auth_header(
                        endpoint,
                        api_key,
                        model,
                        format,
                        auth_header,
                    )
                } else {
                    GenericHttpClient::new(
                        endpoint,
                        api_key,
                        model,
                        format,
                    )
                };
                
                Ok(Some(Arc::new(client)))
            }
        }
    }
    
    /// 确定 API 格式
    /// 
    /// 优先级：
    /// 1. 如果 endpoint 是 compatible-mode，使用 OpenAI 格式
    /// 2. 如果模型包含视觉关键词，使用 QwenVision
    /// 3. 否则使用 driver 的默认格式
    fn determine_api_format(
        profile: &AgentConfig,
        endpoint: &str,
        model: &str,
    ) -> Result<ApiFormat> {
        // 如果 endpoint 是 compatible-mode，统一使用 OpenAI 格式
        if endpoint.contains("compatible-mode") {
            return Ok(ApiFormat::OpenAI);
        }
        
        // 检查是否为视觉模型
        let is_vision = ImageProcessor::is_vision_model(
            Some(model),
            profile.rules.as_ref()
                .and_then(|r| r.payload_building.as_ref())
                .and_then(|p| p.image_processing.as_ref())
                .map(|i| i.vision_keywords.iter().map(|s| s.as_str()).collect::<Vec<_>>())
                .as_deref(),
        );
        
        // 如果是 Qwen 驱动且是视觉模型，使用 QwenVision 格式
        if profile.driver == AgentDriverKind::Qwen && is_vision {
            return Ok(ApiFormat::QwenVision);
        }
        
        // 否则使用 driver 的默认格式
        profile.driver.api_format()
            .ok_or_else(|| AgentFlowError::Other(anyhow!(
                "Unsupported driver: {}. Please provide a valid driver or use 'generic' driver with custom endpoint.",
                profile.driver.as_str()
            )))
    }
    
    /// 获取 API Key（优先从配置读取，后备环境变量）
    fn get_api_key(profile: &AgentConfig) -> Result<String> {
        profile
            .api_key
            .clone()
            .or_else(|| {
                // 如果配置中没有 api_key，尝试从环境变量读取
                profile.driver.default_env_key()
                    .and_then(|env_key| {
                        std::env::var(env_key).ok().map(|key| {
                            tracing::info!(
                                driver = %profile.driver.as_str(),
                                env_key = env_key,
                                "Using API key from environment variable instead of config"
                            );
                            key
                        })
                    })
            })
            .ok_or_else(|| {
                let env_key_hint = profile.driver.default_env_key()
                    .map(|k| format!(" or set environment variable {}", k))
                    .unwrap_or_default();
                AgentFlowError::Other(anyhow::anyhow!(
                    "Missing API key for driver '{}'. Please provide 'api_key' field in agent config (type: 'agent' node){}.",
                    profile.driver.as_str(),
                    env_key_hint
                ))
            })
    }
}

#[cfg(not(feature = "openai-client"))]
impl LlmClientFactory {
    pub fn create_client(_profile: &AgentConfig) -> Result<Option<DynLlmClient>> {
        Ok(None)
    }
}
