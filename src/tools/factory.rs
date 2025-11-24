use std::sync::Arc;

use anyhow::anyhow;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::Value;

use crate::error::Result;
use crate::llm::{DynLlmClient, LlmRequest, LocalEchoClient};
use crate::state::FlowContext;
use crate::tools::tool::{Tool, ToolInvocation};

pub type ToolFactory = Arc<dyn Fn(Option<Value>) -> Result<Arc<dyn Tool>> + Send + Sync>;

#[derive(Default)]
pub struct ToolFactoryRegistry {
    factories: std::collections::HashMap<String, ToolFactory>,
}

impl ToolFactoryRegistry {
    pub fn new() -> Self {
        Self {
            factories: std::collections::HashMap::new(),
        }
    }

    pub fn register_factory<T: Into<String>>(&mut self, name: T, factory: ToolFactory) {
        self.factories.insert(name.into(), factory);
    }

    pub fn build(&self, factory_name: &str, config: Option<Value>) -> Result<Arc<dyn Tool>> {
        let factory = self.factories.get(factory_name).ok_or_else(|| {
            crate::error::AgentFlowError::ToolNotRegistered(factory_name.to_string())
        })?;
        factory(config)
    }
}

fn extract_config<T: DeserializeOwned>(config: Option<Value>) -> Result<T> {
    let normalized = config.unwrap_or_else(|| Value::Object(serde_json::Map::new()));
    serde_json::from_value(normalized).map_err(|e| crate::error::AgentFlowError::Other(anyhow!(e)))
}

pub fn register_builtin_tool_factories(registry: &mut ToolFactoryRegistry) {
    registry.register_factory(
        "echo",
        Arc::new(|config| {
            #[derive(Deserialize)]
            struct Conf {
                #[serde(default = "default_prefix")]
                prefix: String,
            }
            fn default_prefix() -> String {
                "Echo".to_string()
            }
            let conf: Conf = extract_config(config)?;
            Ok(Arc::new(EchoToolWithPrefix {
                prefix: conf.prefix,
            }) as Arc<dyn Tool>)
        }),
    );
    
    registry.register_factory(
        "llm.local_echo",
        Arc::new(|config| {
            #[derive(Deserialize)]
            struct Conf {
                #[serde(default)]
                system_prompt: Option<String>,
                #[serde(default = "default_temperature")]
                temperature: f32,
            }
            fn default_temperature() -> f32 {
                0.2
            }
            let conf: Conf = extract_config(config)?;
            let client: DynLlmClient = Arc::new(LocalEchoClient::default());
            Ok(Arc::new(LlmTool::new(
                "llm.local_echo",
                client,
                LlmToolConfig {
                    system_prompt: conf.system_prompt,
                    temperature: conf.temperature,
                },
            )) as Arc<dyn Tool>)
        }),
    );
    
    registry.register_factory(
        "downloader",
        Arc::new(|_config| {
            Ok(Arc::new(crate::tools::DownloaderTool::new()) as Arc<dyn Tool>)
        }),
    );
    
    registry.register_factory(
        "image_generator",
        Arc::new(|_config| {
            Ok(Arc::new(crate::tools::ImageGeneratorTool::new()) as Arc<dyn Tool>)
        }),
    );
}

struct EchoToolWithPrefix {
    prefix: String,
}

#[async_trait::async_trait]
impl Tool for EchoToolWithPrefix {
    fn name(&self) -> &'static str {
        "echo"
    }

    async fn call(
        &self,
        invocation: ToolInvocation,
        _ctx: &FlowContext,
    ) -> Result<crate::agent::AgentMessage> {
        Ok(crate::agent::AgentMessage {
            id: crate::agent::message::uuid(),
            role: crate::agent::MessageRole::Tool,
            from: self.prefix.clone(),
            to: invocation
                .metadata
                .as_ref()
                .and_then(|v| v.get("reply_to"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            content: format!("{}: {}", self.prefix, invocation.input),
            metadata: invocation.metadata,
        })
    }
}

#[derive(Clone)]
struct LlmToolConfig {
    system_prompt: Option<String>,
    temperature: f32,
}

struct LlmTool {
    name: &'static str,
    client: DynLlmClient,
    config: LlmToolConfig,
}

impl LlmTool {
    fn new(name: &'static str, client: DynLlmClient, config: LlmToolConfig) -> Self {
        Self {
            name,
            client,
            config,
        }
    }

    fn extract_user_input(input: &Value) -> String {
        if let Some(s) = input.as_str() {
            return s.to_string();
        }
        if let Some(obj) = input.as_object() {
            if let Some(content) = obj.get("content").and_then(|v| v.as_str()) {
                return content.to_string();
            }
        }
        input.to_string()
    }
}

#[async_trait::async_trait]
impl Tool for LlmTool {
    fn name(&self) -> &'static str {
        self.name
    }

    async fn call(
        &self,
        invocation: ToolInvocation,
        ctx: &FlowContext,
    ) -> Result<crate::agent::AgentMessage> {
        use tracing::debug;
        let user_input = Self::extract_user_input(&invocation.input);
        let metadata = invocation.metadata.clone();
        let request = LlmRequest {
            system: self.config.system_prompt.clone(),
            user: user_input,
            temperature: self.config.temperature,
            metadata: metadata.clone(),
            image_url: None,
            image_base64: None,
        };
        let response = self.client.complete(request).await?;
        let content = response.content.clone();
        if let Err(err) = ctx.store().set("llm.last_tool", content.clone()).await {
            debug!(%err, "failed to persist llm.last_tool");
        }

        Ok(crate::agent::AgentMessage {
            id: crate::agent::message::uuid(),
            role: crate::agent::MessageRole::Tool,
            from: self.name.to_string(),
            to: None,
            content,
            metadata: response.metadata.or(metadata),
        })
    }
}
