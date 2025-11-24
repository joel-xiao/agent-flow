
use async_trait::async_trait;

use crate::agent::{AgentMessage, MessageRole};
use crate::error::Result;
use crate::state::FlowContext;
use crate::tools::tool::{Tool, ToolInvocation};

pub struct EchoTool;

#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &'static str {
        "echo"
    }

    async fn call(&self, invocation: ToolInvocation, _ctx: &FlowContext) -> Result<AgentMessage> {
        Ok(AgentMessage {
            id: crate::agent::message::uuid(),
            role: MessageRole::Tool,
            from: self.name().to_string(),
            to: invocation
                .metadata
                .as_ref()
                .and_then(|v| v.get("reply_to"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            content: format!("Echo: {}", invocation.input),
            metadata: invocation.metadata,
        })
    }
}
