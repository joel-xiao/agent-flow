use crate::agent::AgentMessage;
use crate::error::{AgentFlowError, Result};
use crate::flow::config::PromptBuildingRules;
use crate::flow::constants::{prompt as prompt_consts, routing as routing_consts};
use anyhow::anyhow;
use serde_json::Value;

/// Prompt 构建服务
pub struct PromptBuilder;

impl PromptBuilder {
    /// 构建系统 prompt
    ///
    /// 根据 role 和 prompt 字段构建系统提示词
    /// 如果提供了 rules 配置，使用配置的模板；否则使用默认模板
    pub fn build_system_prompt(
        role: Option<&str>,
        prompt: Option<&str>,
        rules: Option<&PromptBuildingRules>,
    ) -> Result<String> {
        let system_prompt = if let Some(role) = role {
            if let Some(prompt) = prompt {
                if let Some(r) = rules {
                    r.role_prompt_template
                        .replace("{role}", role)
                        .replace("{prompt}", prompt)
                } else {
                    prompt_consts::TEMPLATE_ROLE_AND_PROMPT
                        .replace("{role}", role)
                        .replace("{prompt}", prompt)
                }
            } else {
                if let Some(r) = rules {
                    r.role_template.replace("{role}", role)
                } else {
                    prompt_consts::TEMPLATE_ROLE_ONLY.replace("{role}", role)
                }
            }
        } else if let Some(prompt) = prompt {
            prompt.to_string()
        } else {
            return Err(AgentFlowError::Other(anyhow!(
                "Missing role or prompt configuration"
            )));
        };

        Ok(system_prompt)
    }

    /// 为自动路由添加路由提示
    ///
    /// 如果提供了自定义路由提示，使用自定义的；否则自动生成
    pub fn add_routing_instructions(
        prompt: &mut String,
        route_targets: &[String],
        custom_route_prompt: Option<&str>,
    ) {
        let route_prompt = if let Some(custom) = custom_route_prompt {
            custom.to_string()
        } else {
            let route_labels: Vec<String> = route_targets
                .iter()
                .filter_map(|target| {
                    let parts: Vec<&str> = target.split('_').collect();
                    parts
                        .iter()
                        .find(|p| {
                            !p.is_empty()
                                && **p != routing_consts::TARGET_PREFIX_NODE
                                && **p != routing_consts::TARGET_SUFFIX_HANDLER
                        })
                        .map(|s| s.to_string())
                })
                .collect();

            if !route_labels.is_empty() {
                format!(
                    "{}{}{}",
                    prompt_consts::ROUTING_INSTRUCTION_PREFIX,
                    prompt_consts::ROUTING_INSTRUCTION_AVAILABLE_ROUTES
                        .replace("{}", &route_labels.join(", ")),
                    prompt_consts::ROUTING_INSTRUCTION_JSON_FORMAT
                )
            } else {
                format!(
                    "{}{}{}",
                    prompt_consts::ROUTING_INSTRUCTION_PREFIX,
                    prompt_consts::ROUTING_INSTRUCTION_AVAILABLE_TARGETS
                        .replace("{}", &route_targets.join(", ")),
                    prompt_consts::ROUTING_INSTRUCTION_JSON_FORMAT_TARGETS
                )
            }
        };

        prompt.push_str(&route_prompt);
    }

    /// 构建完整的系统 prompt（包含路由提示）
    ///
    /// 这是一个便捷方法，组合了 build_system_prompt 和 add_routing_instructions
    pub fn build_system_prompt_with_routing(
        role: Option<&str>,
        prompt: Option<&str>,
        route_mode: Option<&str>,
        route_targets: Option<&[String]>,
        custom_route_prompt: Option<&str>,
        rules: Option<&PromptBuildingRules>,
    ) -> Result<String> {
        let mut system_prompt = Self::build_system_prompt(role, prompt, rules)?;

        if route_mode == Some(routing_consts::MODE_AUTO) {
            if let Some(route_targets) = route_targets {
                Self::add_routing_instructions(
                    &mut system_prompt,
                    route_targets,
                    custom_route_prompt,
                );
            }
        }

        Ok(system_prompt)
    }

    /// 从历史消息中提取前序 Agent 的输出作为上下文
    ///
    /// 这个方法会提取最近几条消息的 response 内容，作为上下文信息
    pub fn extract_history_context(history: &[AgentMessage], max_items: usize) -> String {
        if history.is_empty() {
            return String::new();
        }

        let mut context_parts = Vec::new();
        let start_index = if history.len() > max_items {
            history.len() - max_items
        } else {
            0
        };

        for msg in &history[start_index..] {
            if let Ok(payload) = serde_json::from_str::<Value>(&msg.content) {
                let mut info_parts = Vec::new();
                
                if let Some(last_agent) = payload.get("last_agent").and_then(|v| v.as_str()) {
                    info_parts.push(format!("Agent: {}", last_agent));
                }
                
                if let Some(response) = payload.get("response").and_then(|v| v.as_str()) {
                    if let Ok(json_response) = serde_json::from_str::<Value>(response) {
                        if let Ok(pretty_json) = serde_json::to_string_pretty(&json_response) {
                            info_parts.push(format!("Output: {}", pretty_json));
                        } else {
                            info_parts.push(format!("Output: {}", response));
                        }
                    } else {
                        info_parts.push(format!("Output: {}", response));
                    }
                }

                if !info_parts.is_empty() {
                    context_parts.push(info_parts.join(", "));
                }
            }
        }

        if context_parts.is_empty() {
            String::new()
        } else {
            format!("\n\n<context>\nPrevious analysis steps:\n{}\n</context>", context_parts.join("\n"))
        }
    }

    /// 构建包含历史上下文的系统 prompt
    ///
    /// 在原有的系统 prompt 基础上，附加前序 Agent 的输出作为上下文
    pub fn build_system_prompt_with_history(
        role: Option<&str>,
        prompt: Option<&str>,
        route_mode: Option<&str>,
        route_targets: Option<&[String]>,
        custom_route_prompt: Option<&str>,
        rules: Option<&PromptBuildingRules>,
        history: &[AgentMessage],
        max_history_items: usize,
    ) -> Result<String> {
        let mut system_prompt = Self::build_system_prompt_with_routing(
            role,
            prompt,
            route_mode,
            route_targets,
            custom_route_prompt,
            rules,
        )?;

        let history_context = Self::extract_history_context(history, max_history_items);
        if !history_context.is_empty() {
            system_prompt.push_str(&history_context);
        }

        Ok(system_prompt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_system_prompt_with_role_and_prompt() {
        let prompt = PromptBuilder::build_system_prompt(
            Some("Assistant"),
            Some("You help users with questions."),
            None,
        )
        .unwrap();

        assert!(prompt.contains("You are Assistant"));
        assert!(prompt.contains("You help users with questions"));
    }

    #[test]
    fn test_build_system_prompt_with_role_only() {
        let prompt = PromptBuilder::build_system_prompt(Some("Assistant"), None, None).unwrap();
        assert_eq!(prompt, "You are Assistant.");
    }

    #[test]
    fn test_build_system_prompt_with_prompt_only() {
        let prompt = PromptBuilder::build_system_prompt(None, Some("Help users."), None).unwrap();
        assert_eq!(prompt, "Help users.");
    }

    #[test]
    fn test_build_system_prompt_missing_both() {
        let result = PromptBuilder::build_system_prompt(None, None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_add_routing_instructions() {
        let mut prompt = "You are an assistant.".to_string();
        let route_targets = vec![
            "node_urgent_handler".to_string(),
            "node_normal_handler".to_string(),
        ];

        PromptBuilder::add_routing_instructions(&mut prompt, &route_targets, None);

        assert!(prompt.contains("IMPORTANT"));
        assert!(prompt.contains("urgent"));
        assert!(prompt.contains("normal"));
        assert!(prompt.contains("JSON format"));
    }

    #[test]
    fn test_build_system_prompt_with_routing() {
        let prompt = PromptBuilder::build_system_prompt_with_routing(
            Some("Router"),
            Some("Route requests."),
            Some("auto"),
            Some(&vec!["node_urgent".to_string()]),
            None,
            None,
        )
        .unwrap();

        assert!(prompt.contains("You are Router"));
        assert!(prompt.contains("IMPORTANT"));
        assert!(prompt.contains("urgent"));
    }
}
