pub mod builtin;

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::Result;
use crate::state::FlowContext;
use crate::tools::ToolInvocation;

pub type AgentFactory = Arc<dyn Fn(Option<Value>) -> Result<Arc<dyn Agent>> + Send + Sync>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentMessage {
    pub id: String,
    pub role: MessageRole,
    pub from: String,
    pub to: Option<String>,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

impl AgentMessage {
    pub fn user<T: Into<String>>(content: T) -> Self {
        Self {
            id: uuid(),
            role: MessageRole::User,
            from: "user".into(),
            to: None,
            content: content.into(),
            metadata: None,
        }
    }

    pub fn system<T: Into<String>>(content: T) -> Self {
        Self {
            id: uuid(),
            role: MessageRole::System,
            from: "system".into(),
            to: None,
            content: content.into(),
            metadata: None,
        }
    }

    pub fn tool<T: Into<String>>(name: T, content: T) -> Self {
        Self {
            id: uuid(),
            role: MessageRole::Tool,
            from: name.into(),
            to: None,
            content: content.into(),
            metadata: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    System,
    Assistant,
    Tool,
    Agent,
}

#[derive(Clone)]
pub struct AgentContext<'a> {
    pub flow_ctx: &'a FlowContext,
    pub runtime: &'a dyn AgentRuntime,
}

#[async_trait]
pub trait AgentRuntime: Send + Sync {
    async fn call_tool(&self, name: &str, invocation: ToolInvocation) -> Result<AgentMessage>;
    async fn emit_message(&self, message: AgentMessage) -> Result<()>;
}

#[async_trait]
pub trait Agent: Send + Sync {
    fn name(&self) -> &'static str;
    async fn on_message(
        &self,
        message: AgentMessage,
        ctx: &AgentContext<'_>,
    ) -> Result<AgentAction>;
}

#[derive(Clone, Debug)]
pub enum AgentAction {
    Next {
        target: String,
        message: AgentMessage,
    },
    Branch {
        branches: HashMap<String, AgentMessage>,
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

pub(crate) fn uuid() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards");
    format!("msg-{}-{}", now.as_secs(), now.subsec_nanos())
}

pub type AgentRegistry = HashMap<String, Arc<dyn Agent>>;

pub fn register_agent(name: &str, agent: Arc<dyn Agent>, registry: &mut AgentRegistry) {
    registry.insert(name.to_string(), agent);
}

#[derive(Default)]
pub struct AgentFactoryRegistry {
    factories: HashMap<String, AgentFactory>,
}

impl AgentFactoryRegistry {
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
        }
    }

    pub fn register_factory<T: Into<String>>(&mut self, name: T, factory: AgentFactory) {
        self.factories.insert(name.into(), factory);
    }

    pub fn build(&self, factory_name: &str, config: Option<Value>) -> Result<Arc<dyn Agent>> {
        let builder = self.factories.get(factory_name).ok_or_else(|| {
            crate::error::AgentFlowError::AgentNotRegistered(factory_name.to_string())
        })?;
        builder(config)
    }

    pub fn has_factory(&self, name: &str) -> bool {
        self.factories.contains_key(name)
    }
}
