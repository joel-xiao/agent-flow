use crate::error::{AgentFlowError, Result};
use anyhow::anyhow;

/// 配置验证器
pub struct ConfigValidator;

impl ConfigValidator {
    /// 验证 API Key 格式
    pub fn validate_api_key(api_key: &str) -> Result<()> {
        if api_key.is_empty() {
            return Err(AgentFlowError::Other(anyhow!("API Key 不能为空")));
        }

        if api_key.starts_with("your_") || api_key.starts_with("sk-") && api_key.len() < 20 {
            return Err(AgentFlowError::Other(anyhow!(
                "API Key 看起来是占位符，请提供真实的 API Key"
            )));
        }

        Ok(())
    }

    /// 验证 URL 格式
    pub fn validate_url(url: &str) -> Result<()> {
        if url.is_empty() {
            return Err(AgentFlowError::Other(anyhow!("URL 不能为空")));
        }

        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(AgentFlowError::Other(anyhow!(
                "URL 必须以 http:// 或 https:// 开头"
            )));
        }

        Ok(())
    }

    /// 验证模型名称
    pub fn validate_model_name(model: &str) -> Result<()> {
        if model.is_empty() {
            return Err(AgentFlowError::Other(anyhow!("模型名称不能为空")));
        }

        let lower = model.to_lowercase();
        if lower.contains("gpt") && !lower.contains("gpt-") {
            tracing::warn!(
                model = %model,
                "模型名称可能有误，GPT 模型通常格式为 'gpt-3.5-turbo' 或 'gpt-4'"
            );
        }

        Ok(())
    }

    /// 验证节点 ID
    pub fn validate_node_id(node_id: &str) -> Result<()> {
        if node_id.is_empty() {
            return Err(AgentFlowError::Other(anyhow!("节点 ID 不能为空")));
        }

        if !node_id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return Err(AgentFlowError::Other(anyhow!(
                "节点 ID '{}' 包含无效字符，应该只包含字母、数字、下划线和短横线",
                node_id
            )));
        }

        Ok(())
    }

    /// 验证工作流名称
    pub fn validate_workflow_name(name: &str) -> Result<()> {
        if name.is_empty() {
            return Err(AgentFlowError::Other(anyhow!("工作流名称不能为空")));
        }

        if name.len() > 100 {
            return Err(AgentFlowError::Other(anyhow!(
                "工作流名称过长（最多 100 字符）"
            )));
        }

        Ok(())
    }

    /// 验证温度参数
    pub fn validate_temperature(temperature: f64) -> Result<()> {
        if !(0.0..=2.0).contains(&temperature) {
            return Err(AgentFlowError::Other(anyhow!(
                "温度参数必须在 0.0 到 2.0 之间，当前值: {}",
                temperature
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_api_key() {
        assert!(ConfigValidator::validate_api_key("").is_err());
        assert!(ConfigValidator::validate_api_key("your_api_key_here").is_err());
        assert!(ConfigValidator::validate_api_key("sk-short").is_err());
        assert!(ConfigValidator::validate_api_key("sk-1234567890abcdef1234567890").is_ok());
    }

    #[test]
    fn test_validate_url() {
        assert!(ConfigValidator::validate_url("").is_err());
        assert!(ConfigValidator::validate_url("example.com").is_err());
        assert!(ConfigValidator::validate_url("http://example.com").is_ok());
        assert!(ConfigValidator::validate_url("https://example.com").is_ok());
    }

    #[test]
    fn test_validate_node_id() {
        assert!(ConfigValidator::validate_node_id("").is_err());
        assert!(ConfigValidator::validate_node_id("node-1").is_ok());
        assert!(ConfigValidator::validate_node_id("node_1").is_ok());
        assert!(ConfigValidator::validate_node_id("node@1").is_err());
    }

    #[test]
    fn test_validate_temperature() {
        assert!(ConfigValidator::validate_temperature(-0.1).is_err());
        assert!(ConfigValidator::validate_temperature(0.0).is_ok());
        assert!(ConfigValidator::validate_temperature(1.0).is_ok());
        assert!(ConfigValidator::validate_temperature(2.0).is_ok());
        assert!(ConfigValidator::validate_temperature(2.1).is_err());
    }
}
