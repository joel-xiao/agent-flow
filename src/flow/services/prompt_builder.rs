use crate::error::{Result, AgentFlowError};
use anyhow::anyhow;
use crate::flow::constants::{routing as routing_consts, prompt as prompt_consts};
use crate::flow::config::PromptBuildingRules;

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
                    prompt_consts::TEMPLATE_ROLE_AND_PROMPT.replace("{role}", role).replace("{prompt}", prompt)
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
            // 从路由目标中提取路由标签
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
                    prompt_consts::ROUTING_INSTRUCTION_AVAILABLE_ROUTES.replace("{}", &route_labels.join(", ")),
                    prompt_consts::ROUTING_INSTRUCTION_JSON_FORMAT
                )
            } else {
                format!(
                    "{}{}{}",
                    prompt_consts::ROUTING_INSTRUCTION_PREFIX,
                    prompt_consts::ROUTING_INSTRUCTION_AVAILABLE_TARGETS.replace("{}", &route_targets.join(", ")),
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
        
        // 如果启用自动路由，添加路由提示
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
        let route_targets = vec!["node_urgent_handler".to_string(), "node_normal_handler".to_string()];
        
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

