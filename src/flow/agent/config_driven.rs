use std::sync::Arc;
use async_trait::async_trait;
use serde_json::{Value, json};

use crate::agent::{
    Agent, AgentAction, AgentContext, AgentMessage, MessageRole,
};
use crate::error::{Result, AgentFlowError};
use crate::llm::DynLlmClient;
use crate::tools::Tool;
use crate::{StructuredMessage, ToolInvocation};
use crate::FlowContext;

use crate::flow::config::{AgentConfig, ToolConfig};
use crate::flow::config::driver::AgentDriverKind;
use crate::flow::config::agent::ToolDriverKind;
use crate::flow::services::message_parser::MessageParser;
use crate::flow::services::image_processor::{ImageProcessor, ImageInfo};
use crate::flow::services::prompt_builder::PromptBuilder;
use crate::flow::services::routing::{RouteMatcher, clean_response};
use crate::flow::services::llm_caller::LlmCaller;
use crate::flow::constants::{fields, routing as routing_consts};

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
        
        // 获取业务规则配置
        let rules = self.profile.rules.as_ref();
        let field_extraction_rules = rules.and_then(|r| r.field_extraction.as_ref());
        let prompt_building_rules = rules.and_then(|r| r.prompt_building.as_ref());
        let routing_rules = rules.and_then(|r| r.routing.as_ref());
        let payload_building_rules = rules.and_then(|r| r.payload_building.as_ref());
        
        // 1. 解析消息 payload 和 steps
        let mut payload = MessageParser::parse_payload(&message, &history)?;
        let steps_field = field_extraction_rules.map(|r| r.steps_field.as_str());
        let mut steps = MessageParser::extract_steps(&payload, &history, steps_field)?;
        
        // 2. 处理图像信息
        let vision_keywords = payload_building_rules
            .and_then(|r| r.image_processing.as_ref())
            .map(|r| r.vision_keywords.iter().map(|s| s.as_str()).collect::<Vec<_>>());
        let is_vision_agent = ImageProcessor::is_vision_model(
            self.profile.model.as_deref(),
            vision_keywords.as_deref(),
        );
        let mut image_info = ImageProcessor::extract_image_info(&payload, is_vision_agent)?;
        
        // 处理图像路径转换为 base64
        if let Some(path) = &image_info.path {
            if let Ok(base64_data) = ImageProcessor::process_image_path(path) {
                image_info.base64 = Some(base64_data);
            }
        }
        
        let image_base64_final = ImageProcessor::get_final_base64(&image_info)?;

        // 3. 调用 LLM 或使用原始输入
        #[cfg(feature = "openai-client")]
        let response_content = LlmCaller::call_llm_or_get_raw(
            self.llm_client.as_ref(),
            &payload,
            &history,
            &image_info,
            image_base64_final.clone(),
            &self.profile,
            field_extraction_rules,
            prompt_building_rules,
        ).await?;
        
        #[cfg(not(feature = "openai-client"))]
        let response_content = LlmCaller::get_raw_from_payload(&payload)?;
        
        // 清理响应内容（提取 JSON，处理代码块包裹的情况）
        let response_content_clean = if self.profile.route_mode.as_deref() == Some(routing_consts::MODE_AUTO) {
            clean_response(&response_content, routing_rules)
        } else {
            response_content.clone()
        };

        // 4. 构建响应 payload
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
        
        // 处理图像字段
        if is_vision_agent {
            ImageProcessor::add_image_fields(&mut payload, &image_info);
        } else {
            ImageProcessor::remove_image_fields(&mut payload);
        }
        
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

        // 4.5. 从响应中提取字段并设置到状态存储（供 Decision 节点使用）
        // 尝试解析响应内容为 JSON，提取字段并设置到状态存储
        if let Ok(response_json) = serde_json::from_str::<Value>(&response_content_clean) {
            if response_json.is_object() {
                // 提取常见字段到状态存储（供 Decision 节点使用）
                // 这些字段会从 LLM 的 JSON 响应中提取，并设置到状态存储中
                let fields_to_extract = ["food_count", "route", "image_quality"];
                for field_name in &fields_to_extract {
                    if let Some(value) = response_json.get(field_name) {
                        let value_str = match value {
                            Value::String(s) => s.clone(),
                            Value::Number(n) => n.to_string(),
                            Value::Bool(b) => b.to_string(),
                            _ => value.to_string(),
                        };
                        let value_str_clone = value_str.clone();
                        // 设置到状态存储
                        match ctx.flow_ctx.store().set(field_name, value_str).await {
                            Ok(_) => {
                                tracing::debug!(
                                    agent = %self.profile.name,
                                    key = %field_name,
                                    value = %value_str_clone,
                                    "Extracted field and set to state store"
                                );
                            }
                            Err(e) => {
                                tracing::warn!(
                                    agent = %self.profile.name,
                                    key = %field_name,
                                    error = ?e,
                                    "Failed to set extracted field to state store"
                                );
                            }
                        }
                    }
                }
            }
        }

        // 5. 处理自动路由
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
                    // 如果找到匹配的路由，返回 Branch action
                    return Ok(AgentAction::Branch { branches });
                }
                
                // 如果启用了自动路由但解析失败，记录警告并继续
                tracing::warn!(
                    agent = %self.profile.name,
                    "Auto-routing enabled but no valid route found in LLM response. Falling back to Continue action."
                );
            }
        }

        // 6. 返回 Continue action
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

