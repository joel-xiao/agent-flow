use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::agent::AgentMessage;
use crate::error::Result;
use crate::state::FlowContext;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolInvocation {
    pub name: String,
    pub input: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

impl ToolInvocation {
    pub fn new<T: Into<String>>(name: T, input: Value) -> Self {
        Self {
            name: name.into(),
            input,
            metadata: None,
        }
    }
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    async fn call(&self, invocation: ToolInvocation, ctx: &FlowContext) -> Result<AgentMessage>;
}

