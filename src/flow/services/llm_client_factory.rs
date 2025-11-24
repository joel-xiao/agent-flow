use crate::config::EnvConfig;
use crate::error::{AgentFlowError, Result};
use crate::flow::config::AgentConfig;
use crate::flow::config::AgentDriverKind;
#[cfg(feature = "openai-client")]
use crate::llm::ApiFormat;
use crate::llm::DynLlmClient;
#[cfg(feature = "openai-client")]
use crate::GenericHttpClient;
use anyhow::anyhow;
use std::sync::Arc;

/// LLM 客户端工厂
///
/// ## v2.0 完全配置驱动设计
///
/// 所有配置都从JSON读取，无任何硬编码。
///
/// ## 配置要求
///
/// **必需字段**:
/// - `driver`: 驱动标识（如 "qwen", "chatgpt", "generic"）
/// - `model`: 模型名称（如 "qwen-max", "gpt-4"）
/// - `endpoint`: API端点URL
/// - `api_key`: API密钥（支持环境变量引用 ${VAR_NAME}）
///
/// **可选字段**:
/// - `api_format`: API格式（"openai", "qwen", "qwenvision"），不指定则自动推断
/// - `metadata.auth_header`: 自定义认证header（如 "Bearer", "X-API-Key"）
///
/// ## 示例配置
///
/// ```json
/// {
///   "driver": "qwen",
///   "model": "qwen-max",
///   "endpoint": "https://dashscope.aliyuncs.com/compatible-mode/v1",
///   "api_key": "${QWEN_API_KEY}"
/// }
/// ```
///
/// ## 配置任意LLM提供商
///
/// ```json
/// {
///   "driver": "generic",
///   "model": "your-model-name",
///   "endpoint": "https://your-api.com/v1/chat/completions",
///   "api_key": "${YOUR_API_KEY}",
///   "api_format": "openai"
/// }
/// ```
///
/// ## 错误处理
///
/// - **缺少endpoint**: 必须在配置中提供endpoint
/// - **缺少model**: 必须在配置中提供model
/// - **缺少API key**: 必须在配置中提供或设置环境变量
/// - **无法推断格式**: 在metadata中添加 "api_format" 字段
pub struct LlmClientFactory;

#[cfg(feature = "openai-client")]
impl LlmClientFactory {
    /// 创建 LLM 客户端
    ///
    /// ## 参数
    ///
    /// - `profile`: Agent 配置（从JSON加载）
    ///
    /// ## 返回值
    ///
    /// - `Ok(Some(client))`: 成功创建客户端
    /// - `Ok(None)`: Echo 驱动，不需要真实客户端
    /// - `Err(_)`: 配置错误或创建失败
    pub fn create_client(profile: &AgentConfig) -> Result<Option<DynLlmClient>> {
        match profile.driver {
            AgentDriverKind::Echo => Ok(None),
            _ => {
                let api_key = Self::get_api_key(profile)?;

                let model = profile.model.clone().ok_or_else(|| {
                    AgentFlowError::Other(anyhow!(
                        "Missing 'model' field in agent config for driver '{}'.\n\
                         Please add: \"model\": \"your-model-name\"",
                        profile.driver.as_str()
                    ))
                })?;

                let endpoint = profile.endpoint.clone().ok_or_else(|| {
                    AgentFlowError::Other(anyhow!(
                        "Missing 'endpoint' field in agent config for driver '{}'.\n\
                         \n\
                         All LLM configurations must be explicitly provided in JSON.\n\
                         \n\
                         Please add to your agent config:\n\
                         \"endpoint\": \"https://your-api-endpoint.com/v1\"\n\
                         \n\
                         Common endpoints:\n\
                         - 通义千问 (compatible-mode): \"https://dashscope.aliyuncs.com/compatible-mode/v1\"\n\
                         - 通义千问 (native): \"https://dashscope.aliyuncs.com/api/v1/services/aigc/text-generation/generation\"\n\
                         - OpenAI: \"https://api.openai.com/v1\"\n\
                         - Moonshot: \"https://api.moonshot.cn/v1\"\n\
                         - DeepSeek: \"https://api.deepseek.com\"\n\
                         - Any OpenAI-compatible service: specify your endpoint",
                        profile.driver.as_str()
                    ))
                })?;

                let format = Self::determine_api_format(profile, &endpoint, &model)?;

                let auth_header = profile
                    .metadata
                    .as_ref()
                    .and_then(|m| m.get("auth_header"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let client = if let Some(auth_header) = auth_header {
                    GenericHttpClient::with_auth_header(
                        endpoint,
                        api_key,
                        model,
                        format,
                        auth_header,
                    )
                } else {
                    GenericHttpClient::new(endpoint, api_key, model, format)
                };

                Ok(Some(Arc::new(client)))
            }
        }
    }

    /// 确定 API 格式
    ///
    /// 优先级：
    /// 1. metadata.api_format（显式配置）
    /// 2. 从endpoint URL自动推断
    /// 3. 报错要求明确配置
    fn determine_api_format(
        profile: &AgentConfig,
        endpoint: &str,
        model: &str,
    ) -> Result<ApiFormat> {
        if let Some(metadata) = &profile.metadata {
            if let Some(format_value) = metadata.get("api_format") {
                if let Some(format_str) = format_value.as_str() {
                    if let Some(format) = ApiFormat::from_str(format_str) {
                        tracing::info!(
                            driver = %profile.driver.as_str(),
                            format = format_str,
                            "Using api_format from config"
                        );
                        return Ok(format);
                    } else {
                        return Err(AgentFlowError::Other(anyhow!(
                            "Invalid api_format '{}'. Supported formats: 'openai', 'qwen', 'qwenvision'",
                            format_str
                        )));
                    }
                }
            }
        }

        if let Some(format) = ApiFormat::infer_from_endpoint(endpoint, Some(model)) {
            tracing::info!(
                driver = %profile.driver.as_str(),
                endpoint = %endpoint,
                format = %format.as_str(),
                "API format inferred from endpoint"
            );
            return Ok(format);
        }

        Err(AgentFlowError::Other(anyhow!(
            "Unable to infer API format for endpoint '{}' and model '{}'.\n\
             \n\
             Please explicitly specify the format in your agent config's metadata:\n\
             \"metadata\": {{\n\
               \"api_format\": \"openai\"  // or \"qwen\", \"qwenvision\"\n\
             }}\n\
             \n\
             Supported formats:\n\
             - \"openai\": OpenAI-compatible API (most common)\n\
             - \"qwen\": 通义千问原生API\n\
             - \"qwenvision\": 通义千问视觉模型",
            endpoint,
            model
        )))
    }

    /// 获取 API Key
    ///
    /// ## 优先级
    ///
    /// 1. **配置中的 api_key**
    ///    - 支持直接值：`"sk-xxxxx"`
    ///    - 支持环境变量引用：`"${QWEN_API_KEY}"`
    ///
    /// 2. **驱动默认环境变量**（如果配置中无 api_key）
    ///    - 从 `driver.default_env_key()` 读取
    ///    - 例如：qwen → QWEN_API_KEY
    ///
    /// ## 安全性
    ///
    /// - ✅ **推荐**：使用环境变量引用 `"${VAR_NAME}"`
    /// - ❌ **不推荐**：硬编码 API Key 在配置文件中
    /// - ⚠️ **警告**：确保配置文件不被提交到版本控制
    fn get_api_key(profile: &AgentConfig) -> Result<String> {
        let api_key = profile.api_key.as_ref().ok_or_else(|| {
            AgentFlowError::Other(anyhow::anyhow!(
                "Missing 'api_key' field in agent config for driver '{}'",
                profile.driver.as_str()
            ))
        })?;

        let default_env_var = profile.driver.default_env_key().ok_or_else(|| {
            AgentFlowError::Other(anyhow::anyhow!(
                "No default environment variable for driver '{}'",
                profile.driver.as_str()
            ))
        })?;

        EnvConfig::get_api_key(api_key, default_env_var)
    }
}

#[cfg(not(feature = "openai-client"))]
impl LlmClientFactory {
    pub fn create_client(_profile: &AgentConfig) -> Result<Option<DynLlmClient>> {
        Ok(None)
    }
}
