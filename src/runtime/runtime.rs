use async_trait::async_trait;
use std::sync::Arc;

use crate::agent::AgentMessage;
use crate::error::Result;
use crate::state::FlowContext;
use crate::tools::{ToolInvocation, ToolRegistry};

/// Executor 运行时实现
pub struct ExecutorRuntime {
    pub ctx: Arc<FlowContext>,
    pub tools: Arc<ToolRegistry>,
}

#[async_trait]
impl crate::agent::AgentRuntime for ExecutorRuntime {
    async fn call_tool(&self, name: &str, invocation: ToolInvocation) -> Result<AgentMessage> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| crate::error::AgentFlowError::ToolNotRegistered(name.to_string()))?;
        let response = tool.call(invocation, &self.ctx).await?;
        Ok(response)
    }

    async fn emit_message(&self, message: AgentMessage) -> Result<()> {
        self.ctx.push_message(message);
        Ok(())
    }
}
