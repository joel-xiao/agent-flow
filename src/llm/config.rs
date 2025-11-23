use std::collections::HashMap;

/// API 端点配置
/// 
/// 用于配置 LLM API 的端点和请求头
pub struct ApiEndpointConfig {
    pub base_url: String,
    pub endpoints: HashMap<String, String>,
    pub auth_header: Option<String>,
    pub default_headers: HashMap<String, String>,
}

impl ApiEndpointConfig {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            endpoints: HashMap::new(),
            auth_header: None,
            default_headers: HashMap::new(),
        }
    }

    pub fn with_endpoint(mut self, name: &str, path: &str) -> Self {
        self.endpoints.insert(name.to_string(), path.to_string());
        self
    }

    pub fn with_auth_header(mut self, header: &str) -> Self {
        self.auth_header = Some(header.to_string());
        self
    }

    pub fn with_default_header(mut self, key: &str, value: &str) -> Self {
        self.default_headers.insert(key.to_string(), value.to_string());
        self
    }

    pub fn get_endpoint(&self, name: &str) -> Option<String> {
        self.endpoints.get(name).map(|path| {
            format!("{}{}", self.base_url, path)
        })
    }
}

