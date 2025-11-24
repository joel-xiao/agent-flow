use std::collections::HashMap;
use std::time::Duration;

use anyhow::anyhow;
use futures::future;
use serde_json::Value;
use tokio::time::timeout;
use tracing::{info, warn};

use crate::agent::{AgentMessage, MessageRole};
use crate::error::{AgentFlowError, Result};
use crate::state::FlowContext;

use super::{ToolInvocation, ToolManifest, ToolRegistry};

#[derive(Clone, Debug)]
pub enum ToolStrategy {
    Sequential(Vec<ToolStep>),
    Parallel(Vec<ToolStep>),
    Fallback(Vec<ToolStep>),
}

#[derive(Clone, Debug)]
pub struct ToolStep {
    pub tool: String,
    pub input: Value,
    pub timeout: Option<Duration>,
    pub retries: u32,
    pub name: Option<String>,
}

impl ToolStep {
    pub fn new(tool: impl Into<String>, input: Value) -> Self {
        Self {
            tool: tool.into(),
            input,
            timeout: None,
            retries: 0,
            name: None,
        }
    }

    pub fn with_timeout(mut self, duration: Duration) -> Self {
        self.timeout = Some(duration);
        self
    }

    pub fn with_retries(mut self, retries: u32) -> Self {
        self.retries = retries;
        self
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
}

#[derive(Clone, Debug)]
pub struct ToolPipeline {
    pub name: String,
    pub strategy: ToolStrategy,
    pub output_manifest: Option<ToolManifest>,
}

impl ToolPipeline {
    pub fn new(name: impl Into<String>, strategy: ToolStrategy) -> Self {
        Self {
            name: name.into(),
            strategy,
            output_manifest: None,
        }
    }

    pub fn with_output_manifest(mut self, manifest: ToolManifest) -> Self {
        self.output_manifest = Some(manifest);
        self
    }
}

#[derive(Default)]
pub struct ToolOrchestrator {
    registry: ToolRegistry,
    pipelines: HashMap<String, ToolPipeline>,
}

impl ToolOrchestrator {
    pub fn new(registry: ToolRegistry) -> Self {
        Self {
            registry,
            pipelines: HashMap::new(),
        }
    }

    pub fn register_pipeline(&mut self, pipeline: ToolPipeline) -> Result<()> {
        self.pipelines.insert(pipeline.name.clone(), pipeline);
        Ok(())
    }

    pub fn registry(&self) -> &ToolRegistry {
        &self.registry
    }

    pub fn registry_mut(&mut self) -> &mut ToolRegistry {
        &mut self.registry
    }

    pub async fn execute_pipeline(&self, name: &str, ctx: &FlowContext) -> Result<AgentMessage> {
        self.execute_pipeline_with_params(name, serde_json::json!({}), ctx).await
    }

    pub async fn execute_pipeline_with_params(
        &self,
        name: &str,
        params: Value,
        ctx: &FlowContext,
    ) -> Result<AgentMessage> {
        let pipeline = self
            .pipelines
            .get(name)
            .ok_or_else(|| AgentFlowError::ToolNotRegistered(name.to_string()))?;
        info!(pipeline = %pipeline.name, "executing tool pipeline with params");
        
        let message = self.execute_strategy_with_params(&pipeline.strategy, params, ctx).await?;

        if let Some(manifest) = &pipeline.output_manifest {
            self.validate_output(manifest, &message)?;
        }

        Ok(message)
    }

    pub async fn execute_strategy(
        &self,
        strategy: &ToolStrategy,
        ctx: &FlowContext,
    ) -> Result<AgentMessage> {
        self.execute_strategy_with_params(strategy, serde_json::json!({}), ctx).await
    }

    pub async fn execute_strategy_with_params(
        &self,
        strategy: &ToolStrategy,
        params: Value,
        ctx: &FlowContext,
    ) -> Result<AgentMessage> {
        match strategy {
            ToolStrategy::Sequential(steps) => {
                let mut last_message = AgentMessage::system("tool.pipeline.start");
                for step in steps {
                    let mut merged_input = step.input.clone();
                    if let Some(obj) = merged_input.as_object_mut() {
                        if let Some(params_obj) = params.as_object() {
                            for (k, v) in params_obj {
                                obj.entry(k.clone()).or_insert(v.clone());
                            }
                        }
                    }
                    
                    let mut merged_step = step.clone();
                    merged_step.input = merged_input;
                    last_message = self.execute_step(&merged_step, ctx).await?;
                }
                Ok(last_message)
            }
            ToolStrategy::Parallel(steps) => {
                let futures = steps.iter().map(|step| self.execute_step(step, ctx));
                let results = future::join_all(futures).await;
                let mut messages = Vec::new();
                for result in results {
                    messages.push(result?);
                }
                let aggregated = serde_json::to_string(&messages)
                    .map_err(|e| AgentFlowError::Serialization(e.to_string()))?;
                Ok(AgentMessage::system(aggregated))
            }
            ToolStrategy::Fallback(steps) => {
                let mut last_error: Option<AgentFlowError> = None;
                for step in steps {
                    match self.execute_step(step, ctx).await {
                        Ok(message) => return Ok(message),
                        Err(err) => {
                            warn!(tool = %step.tool, error = ?err, "fallback step failed");
                            last_error = Some(err);
                        }
                    }
                }
                Err(last_error.ok_or_else(|| AgentFlowError::Other(anyhow!("All fallback steps failed")))?)
            }
        }
    }

    async fn execute_step(&self, step: &ToolStep, ctx: &FlowContext) -> Result<AgentMessage> {
        let tool = self
            .registry
            .get(&step.tool)
            .ok_or_else(|| AgentFlowError::ToolNotRegistered(step.tool.clone()))?;

        if let Some(manifest) = self.registry.manifest(&step.tool) {
            self.validate_input(&manifest, &step.input)?;
        }

        let invocation = ToolInvocation {
            name: tool.name().to_string(),
            input: step.input.clone(),
            metadata: None,
        };

        let mut attempts = 0u32;
        loop {
            attempts += 1;
            let task = tool.call(invocation.clone(), ctx);
            let result = if let Some(timeout_duration) = step.timeout {
                match timeout(timeout_duration, task).await {
                    Ok(result) => result,
                    Err(_) => {
                        warn!(tool = %step.tool, "tool invocation timed out");
                        if attempts > step.retries {
                            return Err(AgentFlowError::Other(anyhow::anyhow!(
                                "tool `{}` timed out",
                                step.tool
                            )));
                        } else {
                            continue;
                        }
                    }
                }
            } else {
                task.await
            };

            match result {
                Ok(message) => return Ok(message),
                Err(err) if attempts <= step.retries => {
                    warn!(
                        tool = %step.tool,
                        attempt = attempts,
                        retries = step.retries,
                        error = ?err,
                        "tool invocation failed, retrying"
                    );
                    continue;
                }
                Err(err) => return Err(err),
            }
        }
    }

    fn validate_input(&self, manifest: &ToolManifest, input: &Value) -> Result<()> {
        if manifest.inputs.is_empty() {
            return Ok(());
        }
        if !input.is_object() {
            warn!(
                tool = %manifest.name,
                ?input,
                "tool input not structured; schema enforcement not yet implemented"
            );
        }
        Ok(())
    }

    fn validate_output(&self, manifest: &ToolManifest, message: &AgentMessage) -> Result<()> {
        if manifest.outputs.is_empty() {
            return Ok(());
        }
        if message.role != MessageRole::Tool {
            warn!(
                tool = %manifest.name,
                role = ?message.role,
                "pipeline output role mismatch; expected tool message"
            );
        }
        Ok(())
    }
}
