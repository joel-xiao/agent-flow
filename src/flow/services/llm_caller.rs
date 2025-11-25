use super::message_parser::MessageParser;
use super::prompt_builder::PromptBuilder;
use crate::agent::AgentMessage;
use crate::error::{AgentFlowError, Result};
use crate::flow::config::{AgentConfig, FieldExtractionRules, PromptBuildingRules};
use crate::flow::constants::{fields, llm as llm_consts};
use crate::llm::DynLlmClient;
use crate::LlmRequest;
use futures::StreamExt;
use serde_json::Value;
use std::io::{self, Write};

/// LLM 调用服务
///
/// 负责处理 LLM 调用逻辑，包括 prompt 构建、请求发送和流式响应处理
pub struct LlmCaller;

impl LlmCaller {
    /// 调用 LLM 获取响应
    ///
    /// 如果提供了 LLM 客户端，则调用 LLM；否则从 payload 中提取 raw 字段
    pub async fn call_llm_or_get_raw(
        llm_client: Option<&DynLlmClient>,
        payload: &Value,
        history: &[AgentMessage],
        profile: &AgentConfig,
        field_extraction_rules: Option<&FieldExtractionRules>,
        prompt_building_rules: Option<&PromptBuildingRules>,
    ) -> Result<String> {
        if let Some(llm_client) = llm_client {
            Self::call_llm(
                llm_client,
                payload,
                history,
                profile,
                field_extraction_rules,
                prompt_building_rules,
            )
            .await
        } else {
            Self::get_raw_from_payload(payload)
        }
    }

    /// 调用 LLM
    async fn call_llm(
        llm_client: &DynLlmClient,
        payload: &Value,
        history: &[AgentMessage],
        profile: &AgentConfig,
        field_extraction_rules: Option<&FieldExtractionRules>,
        prompt_building_rules: Option<&PromptBuildingRules>,
    ) -> Result<String> {
        let user_input_fields: Option<Vec<String>> =
            field_extraction_rules.map(|r| r.user_input_fields.clone());
        let user_input =
            MessageParser::extract_user_input(payload, history, user_input_fields.as_deref())?;

        let max_history_items = prompt_building_rules
            .and_then(|r| r.max_history_items)
            .unwrap_or(3);
        
        let system_prompt = PromptBuilder::build_system_prompt_with_history(
            profile.role.as_deref(),
            profile.prompt.as_deref(),
            profile.route_mode.as_deref(),
            profile.route_targets.as_deref(),
            profile.route_prompt.as_deref(),
            prompt_building_rules,
            history,
            max_history_items,
        )?;

        let temperature = prompt_building_rules
            .map(|r| r.temperature)
            .or(profile.temperature)
            .unwrap_or(llm_consts::DEFAULT_TEMPERATURE);

        let llm_request = LlmRequest {
            system: Some(system_prompt),
            user: user_input.to_string(),
            temperature,
            metadata: None,
            image_url: None,
            image_base64: None,
        };

        let role_name = profile.role.as_deref().unwrap_or(&profile.name);

        eprintln!("\n[{}] ⏳ 正在调用 LLM，等待响应...", role_name);
        io::stderr().flush().ok();
        io::stdout().flush().ok();

        println!("\n[{}] 📝 响应内容:", role_name);
        io::stdout().flush().ok();

        print!("  ");
        io::stdout().flush().ok();

        let mut stream = llm_client.complete_stream(llm_request);
        let mut full_response = String::new();
        let mut has_output = false;

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    if !chunk.content.is_empty() {
                        print!("{}", chunk.content);
                        io::stdout().flush().map_err(|e| {
                            AgentFlowError::Other(anyhow::anyhow!("Failed to flush stdout: {}", e))
                        })?;
                        full_response.push_str(&chunk.content);
                        has_output = true;
                    }
                    if chunk.done {
                        if has_output {
                            println!();
                            io::stdout().flush().ok();
                        }
                        eprintln!("[{}] ✅ 响应完成", role_name);
                        io::stderr().flush().ok();
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("\n[{}] ❌ 流式输出错误: {}", role_name, e);
                    io::stderr().flush().ok();
                    return Err(e);
                }
            }
        }

        if !has_output {
            eprintln!("[{}] ⚠️  警告: 没有接收到任何响应内容", role_name);
            io::stderr().flush().ok();
        }

        io::stdout().flush().ok();
        io::stderr().flush().ok();

        Ok(full_response)
    }

    /// 从 payload 中获取 raw 字段
    pub fn get_raw_from_payload(payload: &Value) -> Result<String> {
        payload
            .get(fields::RAW)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| {
                AgentFlowError::Other(anyhow::anyhow!("Missing raw field and no LLM client"))
            })
    }
}
