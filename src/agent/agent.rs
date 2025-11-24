use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

use crate::error::Result;
use crate::state::{FlowContext, FlowScopeGuard, FlowScopeKind, FlowVariables, SessionContext};
use crate::tools::ToolInvocation;

use super::message::{AgentMessage, MessageRole};

#[derive(Clone)]
pub struct AgentContext<'a> {
    pub flow_ctx: &'a FlowContext,
    pub runtime: &'a dyn AgentRuntime,
}

impl<'a> AgentContext<'a> {
    pub fn flow(&self) -> &'a FlowContext {
        self.flow_ctx
    }

    pub fn session(&self) -> SessionContext {
        self.flow_ctx.session()
    }

    pub fn variables(&self) -> FlowVariables {
        self.flow_ctx.variables()
    }

    pub fn scope(&self, kind: FlowScopeKind) -> FlowScopeGuard {
        self.flow_ctx.scope(kind)
    }
}

#[async_trait]
pub trait AgentRuntime: Send + Sync {
    async fn call_tool(&self, name: &str, invocation: ToolInvocation) -> Result<AgentMessage>;
    async fn emit_message(&self, message: AgentMessage) -> Result<()>;
}

#[async_trait]
pub trait Agent: Send + Sync {
    fn name(&self) -> &'static str;
    async fn on_start(&self, _ctx: &AgentContext<'_>) -> Result<()> {
        Ok(())
    }
    async fn on_message(
        &self,
        message: AgentMessage,
        ctx: &AgentContext<'_>,
    ) -> Result<AgentAction>;
    async fn on_finish(&self, _ctx: &AgentContext<'_>) -> Result<()> {
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub enum AgentAction {
    Next {
        target: String,
        message: AgentMessage,
    },
    Branch {
        branches: std::collections::HashMap<String, AgentMessage>,
    },
    CallTool {
        tool: String,
        invocation: ToolInvocation,
        on_complete: Option<String>,
    },
    Finish {
        message: Option<AgentMessage>,
    },
    Continue {
        message: Option<AgentMessage>,
    },
}

#[derive(Clone, Debug)]
pub struct AgentInput<T> {
    pub value: T,
    pub message: AgentMessage,
}

impl<T> AgentInput<T>
where
    T: DeserializeOwned,
{
    pub fn try_from_message(message: AgentMessage) -> Result<Self> {
        let value = serde_json::from_str(&message.content)
            .map_err(|e| crate::error::AgentFlowError::Serialization(e.to_string()))?;
        Ok(Self { value, message })
    }
}

#[derive(Clone, Debug)]
pub struct AgentOutput<T> {
    pub role: MessageRole,
    pub from: String,
    pub to: Option<String>,
    pub value: T,
    pub metadata: Option<Value>,
}

impl<T> AgentOutput<T>
where
    T: Serialize,
{
    pub fn into_message(self) -> Result<AgentMessage> {
        let content = serde_json::to_string(&self.value)
            .map_err(|e| crate::error::AgentFlowError::Serialization(e.to_string()))?;
        Ok(AgentMessage {
            id: super::message::uuid(),
            role: self.role,
            from: self.from,
            to: self.to,
            content,
            metadata: self.metadata,
        })
    }
}
