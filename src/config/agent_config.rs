use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::error::{Result, AgentFlowError};
use anyhow::anyhow;
use super::graph::GraphNode;
use super::agent_rules::AgentRules;

/// Agent 节点配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub driver: String,
    pub role: String,
    pub prompt: String,
    pub model: String,
    /// Endpoint，优先从配置读取，如果没有则从 service 节点读取
    #[serde(default)]
    pub endpoint: Option<String>,
    /// API Key，优先从配置读取，如果没有则从 service 节点读取或环境变量读取
    #[serde(default)]
    pub api_key: Option<String>,
    pub intent: String,
    pub service: String,
    #[serde(default)]
    pub metadata: Option<Value>,
    /// 路由模式: "auto" 启用自动路由, "manual" 或 None 使用手动路由
    #[serde(default)]
    pub route_mode: Option<String>,
    /// 可路由的目标节点 ID 列表
    #[serde(default)]
    pub route_targets: Option<Vec<String>>,
    /// 路由专用的 prompt, 用于指导 LLM 生成路由标签
    #[serde(default)]
    pub route_prompt: Option<String>,
    /// 默认路由目标（当自动路由失败时使用）
    #[serde(default)]
    pub default_route: Option<String>,
    /// 业务规则配置
    #[serde(default)]
    pub rules: Option<AgentRules>,
}

impl GraphNode {
    /// 尝试解析为 Agent 配置
    pub fn as_agent(&self) -> Result<AgentConfig> {
        if self.node_type != "agent" {
            return Err(AgentFlowError::Other(anyhow!("Node {} is not an agent", self.id)));
        }
        serde_json::from_value(self.config.clone())
            .map_err(|e| AgentFlowError::Other(anyhow!("Failed to parse agent config: {}", e)))
    }
}

