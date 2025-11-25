use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::agent::{Agent, AgentAction, AgentContext, AgentMessage, MessageRole};
use crate::error::Result;
use crate::llm::DynLlmClient;
use crate::tools::Tool;
use crate::FlowContext;
use crate::{StructuredMessage, ToolInvocation};

use crate::flow::config::agent::ToolDriverKind;
use crate::flow::config::{AgentConfig, ToolConfig};
use crate::flow::constants::{fields, routing as routing_consts};
use crate::flow::services::llm_caller::LlmCaller;
use crate::flow::services::message_parser::MessageParser;
use crate::flow::services::routing::{clean_response, RouteMatcher};

/// 配置驱动的 Agent 实现
#[derive(Clone)]
pub struct ConfigDrivenAgent {
    pub profile: Arc<AgentConfig>,
    pub name: &'static str,
    #[cfg(feature = "openai-client")]
    pub llm_client: Option<DynLlmClient>,
}

#[async_trait]
impl Agent for ConfigDrivenAgent {
    fn name(&self) -> &'static str {
        self.name
    }

    async fn on_message(
        &self,
        message: AgentMessage,
        ctx: &AgentContext<'_>,
    ) -> Result<AgentAction> {
        let history = ctx.flow().history();

        let rules = self.profile.rules.as_ref();
        let field_extraction_rules = rules.and_then(|r| r.field_extraction.as_ref());
        let prompt_building_rules = rules.and_then(|r| r.prompt_building.as_ref());
        let routing_rules = rules.and_then(|r| r.routing.as_ref());

        let mut payload = MessageParser::parse_payload(&message, &history)?;
        let steps_field = field_extraction_rules.map(|r| r.steps_field.as_str());
        let mut steps = MessageParser::extract_steps(&payload, &history, steps_field)?;

        #[cfg(feature = "openai-client")]
        let response_content = LlmCaller::call_llm_or_get_raw(
            self.llm_client.as_ref(),
            &payload,
            &history,
            &self.profile,
            field_extraction_rules,
            prompt_building_rules,
        )
        .await?;

        #[cfg(not(feature = "openai-client"))]
        let response_content = LlmCaller::get_raw_from_payload(&payload)?;

        let response_content_clean =
            if self.profile.route_mode.as_deref() == Some(routing_consts::MODE_AUTO) {
                clean_response(&response_content, routing_rules)
            } else {
                response_content.clone()
            };

        
        if let Value::Array(ref mut list) = steps {
            list.push(json!({
                fields::AGENT: self.profile.name,
                fields::INTENT: self.profile.intent.as_ref().cloned().unwrap_or_default(),
                fields::DRIVER: self.profile.driver.as_str(),
            }));
        }
        payload[fields::STEPS] = steps;
        payload[fields::LAST_AGENT] = Value::String(self.profile.name.clone());
        payload[fields::RESPONSE] = Value::String(response_content.clone());

        if let Some(prompt) = &self.profile.prompt {
            payload[fields::PROMPT] = Value::String(prompt.clone());
        }
        if let Some(role) = &self.profile.role {
            payload[fields::ROLE] = Value::String(role.clone());
        }
        if let Some(model) = &self.profile.model {
            payload[fields::MODEL] = Value::String(model.clone());
        }
        if let Some(metadata) = &self.profile.metadata {
            payload[fields::AGENT_METADATA] = metadata.clone();
        }

        if let Ok(response_json) = serde_json::from_str::<Value>(&response_content_clean) {
            if response_json.is_object() {
                if let Some(extract_map) = field_extraction_rules.and_then(|r| r.extract_to_state.as_ref()) {
                    for (response_field, state_key) in extract_map {
                        if let Some(value) = response_json.get(response_field) {
                            let value_str = match value {
                                Value::String(s) => s.clone(),
                                Value::Number(n) => n.to_string(),
                                Value::Bool(b) => b.to_string(),
                                _ => value.to_string(),
                            };
                            let value_str_clone = value_str.clone();
                            match ctx.flow_ctx.store().set(state_key, value_str).await {
                                Ok(_) => {
                                    tracing::debug!(
                                        agent = %self.profile.name,
                                        key = %state_key,
                                        value = %value_str_clone,
                                        "Extracted field and set to state store"
                                    );
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        agent = %self.profile.name,
                                        key = %state_key,
                                        error = ?e,
                                        "Failed to set extracted field to state store"
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        if self.profile.route_mode.as_deref() == Some(routing_consts::MODE_AUTO) {
            if let Some(route_targets) = &self.profile.route_targets {
                let matcher = RouteMatcher::new(
                    route_targets.clone(),
                    self.profile.default_route.clone(),
                    routing_rules,
                );

                if let Some(branches) = matcher.match_route(
                    &response_content,
                    &response_content_clean,
                    &payload,
                    &self.profile.name,
                )? {
                    return Ok(AgentAction::Branch { branches });
                }

                tracing::warn!(
                    agent = %self.profile.name,
                    "Auto-routing enabled but no valid route found in LLM response. Falling back to Continue action."
                );
            }
        }

        let message = StructuredMessage::new(payload).into_agent_message(
            MessageRole::Agent,
            &self.profile.name,
            None,
        )?;

        Ok(AgentAction::Continue {
            message: Some(message),
        })
    }
}

/// 配置驱动的 Tool 实现
#[derive(Clone)]
pub struct ConfigDrivenTool {
    pub profile: Arc<ToolConfig>,
    pub name: &'static str,
}

#[async_trait]
impl Tool for ConfigDrivenTool {
    fn name(&self) -> &'static str {
        self.name
    }

    async fn call(&self, invocation: ToolInvocation, _ctx: &FlowContext) -> Result<AgentMessage> {
        let response = json!({
            "tool": self.profile.name,
            "driver": match self.profile.driver {
                ToolDriverKind::Echo => "echo",
            },
            "input": invocation.input,
        });
        StructuredMessage::new(response).into_agent_message(
            MessageRole::Tool,
            &self.profile.name,
            None,
        )
    }
}
