use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use tracing::{info, warn};

use crate::agent::{
    Agent, AgentAction, AgentContext, AgentFactoryRegistry, AgentMessage, MessageRole,
};
use crate::error::{AgentFlowError, Result};
use crate::tools::ToolInvocation;

pub struct UserProxyAgent {
    next: String,
}

impl UserProxyAgent {
    pub fn new<T: Into<String>>(next: T) -> Self {
        Self { next: next.into() }
    }
}

#[async_trait]
impl Agent for UserProxyAgent {
    fn name(&self) -> &'static str {
        "user_proxy"
    }

    async fn on_message(
        &self,
        message: AgentMessage,
        _ctx: &AgentContext<'_>,
    ) -> Result<AgentAction> {
        Ok(AgentAction::Next {
            target: self.next.clone(),
            message,
        })
    }
}

pub struct CoderAgent {
    reviewer: String,
}

impl CoderAgent {
    pub fn new<T: Into<String>>(reviewer: T) -> Self {
        Self {
            reviewer: reviewer.into(),
        }
    }
}

#[async_trait]
impl Agent for CoderAgent {
    fn name(&self) -> &'static str {
        "coder"
    }

    async fn on_message(
        &self,
        message: AgentMessage,
        ctx: &AgentContext<'_>,
    ) -> Result<AgentAction> {
        let code = format!("fn main() {{ println!(\"{}\"); }}", message.content);
        let reply = AgentMessage {
            id: crate::agent::message::uuid(),
            role: MessageRole::Agent,
            from: self.name().to_string(),
            to: Some(self.reviewer.clone()),
            content: code.clone(),
            metadata: Some(json!({ "source": "coder" })),
        };
        ctx.runtime.emit_message(reply.clone()).await?;
        Ok(AgentAction::Next {
            target: self.reviewer.clone(),
            message: reply,
        })
    }
}

pub struct ReviewerAgent {
    coder: String,
}

impl ReviewerAgent {
    pub fn new<T: Into<String>>(coder: T) -> Self {
        Self {
            coder: coder.into(),
        }
    }
}

#[async_trait]
impl Agent for ReviewerAgent {
    fn name(&self) -> &'static str {
        "reviewer"
    }

    async fn on_message(
        &self,
        message: AgentMessage,
        ctx: &AgentContext<'_>,
    ) -> Result<AgentAction> {
        let needs_fix = !message.content.contains("println!");
        let store = ctx.flow_ctx.store();
        if needs_fix {
            if let Err(err) = store.set("review.status", "needs_fix".into()).await {
                warn!(%err, "Failed to write review.status");
            }
            let feedback = AgentMessage {
                id: crate::agent::message::uuid(),
                role: MessageRole::Agent,
                from: self.name().to_string(),
                to: Some(self.coder.clone()),
                content: "Please add println! call".into(),
                metadata: Some(json!({ "needs_fix": true })),
            };
            Ok(AgentAction::Continue {
                message: Some(feedback),
            })
        } else {
            if let Err(err) = store.set("review.status", "pass".into()).await {
                warn!(%err, "Failed to write review.status");
            }
            let approval = AgentMessage::system("Review completed");
            info!("Code review passed");
            Ok(AgentAction::Continue {
                message: Some(approval),
            })
        }
    }
}

pub struct ToolInvokerAgent {
    tool_name: String,
    next: Option<String>,
}

impl ToolInvokerAgent {
    pub fn new<T: Into<String>>(tool_name: T, next: Option<String>) -> Self {
        Self {
            tool_name: tool_name.into(),
            next,
        }
    }
}

fn extract_config<T: DeserializeOwned>(value: Option<Value>) -> Result<T> {
    let normalized = value.unwrap_or_else(|| Value::Object(serde_json::Map::new()));
    serde_json::from_value(normalized).map_err(|e| AgentFlowError::Other(anyhow!(e)))
}

pub fn register_builtin_agent_factories(registry: &mut AgentFactoryRegistry) {
    registry.register_factory(
        "user_proxy",
        Arc::new(|config| {
            #[derive(serde::Deserialize)]
            struct Conf {
                next: String,
            }
            let conf: Conf = extract_config(config)?;
            Ok(Arc::new(UserProxyAgent::new(conf.next)) as Arc<dyn Agent>)
        }),
    );

    registry.register_factory(
        "coder",
        Arc::new(|config| {
            #[derive(serde::Deserialize)]
            struct Conf {
                reviewer: String,
            }
            let conf: Conf = extract_config(config)?;
            Ok(Arc::new(CoderAgent::new(conf.reviewer)) as Arc<dyn Agent>)
        }),
    );

    registry.register_factory(
        "reviewer",
        Arc::new(|config| {
            #[derive(serde::Deserialize)]
            struct Conf {
                coder: String,
            }
            let conf: Conf = extract_config(config)?;
            Ok(Arc::new(ReviewerAgent::new(conf.coder)) as Arc<dyn Agent>)
        }),
    );

    registry.register_factory(
        "tool_invoker",
        Arc::new(|config| {
            #[derive(serde::Deserialize)]
            struct Conf {
                tool_name: String,
                #[serde(default)]
                next: Option<String>,
            }
            let conf: Conf = extract_config(config)?;
            Ok(Arc::new(ToolInvokerAgent::new(conf.tool_name, conf.next)) as Arc<dyn Agent>)
        }),
    );
}

#[async_trait]
impl Agent for ToolInvokerAgent {
    fn name(&self) -> &'static str {
        "tool_invoker"
    }

    async fn on_message(
        &self,
        message: AgentMessage,
        ctx: &AgentContext<'_>,
    ) -> Result<AgentAction> {
        let invocation =
            ToolInvocation::new(&self.tool_name, json!({ "content": message.content }));
        let tool_response = ctx.runtime.call_tool(&self.tool_name, invocation).await?;
        if let Some(next) = &self.next {
            Ok(AgentAction::Next {
                target: next.clone(),
                message: tool_response,
            })
        } else {
            Ok(AgentAction::Finish {
                message: Some(tool_response),
            })
        }
    }
}
