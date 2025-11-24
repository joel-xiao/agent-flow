use crate::error::{AgentFlowError, Result};
use anyhow::anyhow;
use std::env;

/// 环境变量配置管理
pub struct EnvConfig;

impl EnvConfig {
    /// 获取 API Key，支持从环境变量或配置中获取
    ///
    /// 优先级：
    /// 1. 直接传入的 api_key 参数（如果不以 ${} 包裹）
    /// 2. 环境变量（如果 api_key 以 ${VAR_NAME} 格式）
    /// 3. 返回错误
    pub fn get_api_key(api_key: &str, default_env_var: &str) -> Result<String> {
        if api_key.starts_with("${") && api_key.ends_with("}") {
            let env_var_name = &api_key[2..api_key.len() - 1];
            Self::get_env(env_var_name)
        } else if api_key.is_empty() {
            Self::get_env(default_env_var)
        } else {
            Ok(api_key.to_string())
        }
    }

    /// 从环境变量获取值
    pub fn get_env(key: &str) -> Result<String> {
        env::var(key).map_err(|_| {
            AgentFlowError::Other(anyhow!(
                "环境变量 '{}' 未设置。请在 .env 文件中设置或通过环境变量传递。",
                key
            ))
        })
    }

    /// 获取可选的环境变量
    pub fn get_env_optional(key: &str) -> Option<String> {
        env::var(key).ok()
    }

    /// 检查是否启用调试模式
    pub fn is_debug_mode() -> bool {
        env::var("AGENTFLOW_DEBUG").is_ok()
    }

    /// 获取日志级别
    pub fn get_log_level() -> String {
        env::var("RUST_LOG").expect("RUST_LOG environment variable not set")
    }

}

/// 宏：简化环境变量获取
#[macro_export]
macro_rules! env_var {
    ($key:expr) => {
        $crate::config::EnvConfig::get_env($key)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_api_key_direct() {
        let result = EnvConfig::get_api_key("sk-1234567890abcdef1234567890", "TEST_API_KEY");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "sk-1234567890abcdef1234567890");
    }

    #[test]
    fn test_get_api_key_env_var() {
        env::set_var("TEST_QWEN_KEY", "test_key_value");
        let result = EnvConfig::get_api_key("${TEST_QWEN_KEY}", "FALLBACK_KEY");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test_key_value");
        env::remove_var("TEST_QWEN_KEY");
    }

    #[test]
    fn test_get_api_key_placeholder() {
        env::set_var("DEFAULT_KEY", "default_value");
        let result = EnvConfig::get_api_key("your_api_key_here", "DEFAULT_KEY");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "default_value");
        env::remove_var("DEFAULT_KEY");
    }
}
