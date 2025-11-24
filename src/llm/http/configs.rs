use crate::llm::config::ApiEndpointConfig;

/// 通用配置构建器
/// 
/// 不再硬编码任何特定厂商的配置，所有配置都应该从外部传入。
/// 这个模块只提供配置构建的工具函数。
/// 
/// # 设计原则
/// 
/// 1. **不硬编码**: 不包含任何厂商特定的URL、端点或逻辑
/// 2. **通用性**: 支持任意LLM提供商
/// 3. **可配置**: 所有参数从配置文件或代码传入
/// 
/// # 示例
/// 
/// ```rust
/// use agentflow::llm::http::configs::build_config;
/// 
/// // 从配置文件读取参数后创建配置
/// let config = build_config(
///     "https://api.example.com/v1",
///     vec![
///         ("chat_completion", "/chat/completions"),
///         ("text_embedding", "/embeddings"),
///     ],
/// );
/// ```

/// 从参数构建通用API配置
/// 
/// # 参数
/// 
/// - `base_url`: 基础URL
/// - `endpoints`: 端点列表 (name, path)
/// 
/// # 返回
/// 
/// 配置好的 `ApiEndpointConfig`
pub fn build_config(
    base_url: impl Into<String>,
    endpoints: Vec<(&str, &str)>,
) -> ApiEndpointConfig {
    let mut config = ApiEndpointConfig::new(base_url);
    for (name, path) in endpoints {
        config = config.with_endpoint(name, path);
    }
    config
}

/// 从 JSON 构建配置
pub fn build_config_from_json(value: &serde_json::Value) -> Option<ApiEndpointConfig> {
    let base_url = value["base_url"].as_str()?;
    let mut config = ApiEndpointConfig::new(base_url);
    
    if let Some(endpoints) = value["endpoints"].as_object() {
        for (name, path) in endpoints {
            if let Some(path_str) = path.as_str() {
                config = config.with_endpoint(name, path_str);
            }
        }
    }
    
    if let Some(auth_header) = value["auth_header"].as_str() {
        config = config.with_auth_header(auth_header);
    }
    
    if let Some(headers) = value["default_headers"].as_object() {
        for (key, value) in headers {
            if let Some(value_str) = value.as_str() {
                config = config.with_default_header(key, value_str);
            }
        }
    }
    
    Some(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_config() {
        let config = build_config(
            "https://api.example.com/v1",
            vec![
                ("chat_completion", "/chat/completions"),
                ("text_embedding", "/embeddings"),
            ],
        );
        
        assert_eq!(config.base_url, "https://api.example.com/v1");
        assert_eq!(
            config.get_endpoint("chat_completion"),
            Some("https://api.example.com/v1/chat/completions".to_string())
        );
    }

    #[test]
    fn test_build_config_from_json() {
        let json = serde_json::json!({
            "base_url": "https://api.example.com/v1",
            "endpoints": {
                "chat_completion": "/chat/completions",
                "text_embedding": "/embeddings"
            },
            "auth_header": "Bearer",
            "default_headers": {
                "Accept": "application/json"
            }
        });
        
        let config = build_config_from_json(&json).unwrap();
        assert_eq!(config.base_url, "https://api.example.com/v1");
        assert_eq!(config.auth_header, Some("Bearer".to_string()));
    }
}
