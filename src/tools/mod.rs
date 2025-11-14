pub mod manifest;
pub mod orchestrator;
pub mod resources;

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use tracing::debug;

pub use manifest::{ToolManifest, ToolManifestBuilder, ToolPort, ToolPortSchema};

use crate::agent::{self, AgentMessage, MessageRole};
use crate::error::{AgentFlowError, Result};
use crate::llm::{DynLlmClient, LlmRequest, LocalEchoClient};
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

#[derive(Clone)]
struct ToolEntry {
    tool: Arc<dyn Tool>,
    manifest: Option<Arc<ToolManifest>>,
}

#[derive(Default)]
pub struct ToolRegistry {
    tools: HashMap<String, ToolEntry>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        let _ = self.insert(tool, None);
    }

    pub fn register_with_manifest(
        &mut self,
        tool: Arc<dyn Tool>,
        manifest: ToolManifest,
    ) -> Result<()> {
        self.insert(tool, Some(manifest))
    }

    pub fn register_manifest(&mut self, manifest: ToolManifest) -> Result<()> {
        let entry = self
            .tools
            .get_mut(&manifest.name)
            .ok_or_else(|| AgentFlowError::ToolNotRegistered(manifest.name.clone()))?;
        entry.manifest = Some(Arc::new(manifest));
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).map(|entry| Arc::clone(&entry.tool))
    }

    pub fn manifest(&self, name: &str) -> Option<Arc<ToolManifest>> {
        self.tools
            .get(name)
            .and_then(|entry| entry.manifest.as_ref().map(Arc::clone))
    }

    fn insert(&mut self, tool: Arc<dyn Tool>, manifest: Option<ToolManifest>) -> Result<()> {
        if let Some(ref manifest) = manifest {
            if manifest.name != tool.name() {
                return Err(AgentFlowError::ManifestMismatch {
                    kind: "tool",
                    name: tool.name().to_string(),
                });
            }
        }

        self.tools.insert(
            tool.name().to_string(),
            ToolEntry {
                tool,
                manifest: manifest.map(Arc::new),
            },
        );
        Ok(())
    }
}

pub struct EchoTool;

#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &'static str {
        "echo"
    }

    async fn call(&self, invocation: ToolInvocation, _ctx: &FlowContext) -> Result<AgentMessage> {
        Ok(AgentMessage {
            id: agent::uuid(),
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

pub type ToolFactory = Arc<dyn Fn(Option<Value>) -> Result<Arc<dyn Tool>> + Send + Sync>;

#[derive(Default)]
pub struct ToolFactoryRegistry {
    factories: HashMap<String, ToolFactory>,
}

impl ToolFactoryRegistry {
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
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
}

struct EchoToolWithPrefix {
    prefix: String,
}

#[async_trait]
impl Tool for EchoToolWithPrefix {
    fn name(&self) -> &'static str {
        "echo"
    }

    async fn call(&self, invocation: ToolInvocation, _ctx: &FlowContext) -> Result<AgentMessage> {
        Ok(AgentMessage {
            id: agent::uuid(),
            role: MessageRole::Tool,
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

#[async_trait]
impl Tool for LlmTool {
    fn name(&self) -> &'static str {
        self.name
    }

    async fn call(&self, invocation: ToolInvocation, ctx: &FlowContext) -> Result<AgentMessage> {
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

        Ok(AgentMessage {
            id: agent::uuid(),
            role: MessageRole::Tool,
            from: self.name.to_string(),
            to: None,
            content,
            metadata: response.metadata.or(metadata),
        })
    }
}
