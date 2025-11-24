use super::message_builder::build_route_message;
use super::route_extractor::extract_route_from_text;
use crate::agent::AgentMessage;
use crate::error::Result;
use crate::flow::config::RoutingRules;
use crate::flow::constants::{fields, prompt as prompt_consts};
use serde_json::{json, Value};
use std::collections::HashMap;

/// 路由匹配器
///
/// 负责从 LLM 响应中解析路由信息并匹配到目标节点
pub struct RouteMatcher {
    route_targets: Vec<String>,
    default_route: Option<String>,
    routing_rules: Option<RoutingRules>,
}

impl RouteMatcher {
    /// 创建新的路由匹配器
    pub fn new(
        route_targets: Vec<String>,
        default_route: Option<String>,
        routing_rules: Option<&RoutingRules>,
    ) -> Self {
        Self {
            route_targets,
            default_route,
            routing_rules: routing_rules.cloned(),
        }
    }

    /// 从 LLM 响应中解析并匹配路由
    ///
    /// 返回匹配到的路由分支，如果没有匹配则返回 None
    pub fn match_route(
        &self,
        _response: &str,
        response_clean: &str,
        payload: &Value,
        agent_name: &str,
    ) -> Result<Option<HashMap<String, AgentMessage>>> {
        let route_payload_result = serde_json::from_str::<Value>(response_clean);

        let route_payload = if let Ok(parsed) = route_payload_result {
            Some(parsed)
        } else {
            extract_route_from_text(
                response_clean,
                &self.route_targets,
                self.routing_rules.as_ref(),
            )
        };

        if let Some(route_payload) = route_payload {
            if let Some(route) = route_payload.get(fields::ROUTE) {
                let branches = self.process_route(route, &route_payload, payload, agent_name)?;

                if !branches.is_empty() {
                    return Ok(Some(branches));
                }
            }
        }

        if let Some(default_route) = &self.default_route {
            if self.route_targets.contains(default_route) {
                let branches = self.create_default_route(payload, agent_name, default_route)?;
                return Ok(Some(branches));
            }
        }

        Ok(None)
    }

    /// 处理路由标签（可能是字符串或数组）
    fn process_route(
        &self,
        route: &Value,
        route_payload: &Value,
        payload: &Value,
        agent_name: &str,
    ) -> Result<HashMap<String, AgentMessage>> {
        let mut branches = HashMap::new();

        if let Some(route_str) = route.as_str() {
            if let Some(branch) =
                self.match_single_route(route_str, route_payload, payload, agent_name)?
            {
                branches.extend(branch);
            }
        }
        else if let Some(route_array) = route.as_array() {
            let route_labels: Vec<String> = route_array
                .iter()
                .filter_map(|r| r.as_str().map(|s| s.to_string()))
                .collect();

            let multi_branches =
                self.match_multiple_routes(&route_labels, route_payload, payload, agent_name)?;

            branches.extend(multi_branches);
        }

        Ok(branches)
    }

    /// 匹配单个路由标签
    fn match_single_route(
        &self,
        route_str: &str,
        route_payload: &Value,
        payload: &Value,
        agent_name: &str,
    ) -> Result<Option<HashMap<String, AgentMessage>>> {
        let route_label = route_str.to_lowercase();

        for target in &self.route_targets {
            if super::route_matcher_utils::is_route_match(
                &route_label,
                target,
                &self.routing_rules,
                &self.default_route,
            ) {
                let route_message = build_route_message(
                    target,
                    route_str,
                    route_payload.get(fields::ROUTE_REASON),
                    None,
                    payload,
                    agent_name,
                )?;

                let mut branches = HashMap::new();
                branches.insert(target.clone(), route_message);
                return Ok(Some(branches));
            }
        }

        Ok(None)
    }

    /// 匹配多个路由标签（数组）
    fn match_multiple_routes(
        &self,
        route_labels: &[String],
        route_payload: &Value,
        payload: &Value,
        agent_name: &str,
    ) -> Result<HashMap<String, AgentMessage>> {
        let mut branches = HashMap::new();

        for route_str in route_labels {
            let route_label = route_str.to_lowercase();

            for target in &self.route_targets {
                if super::route_matcher_utils::is_route_match(
                    &route_label,
                    target,
                    &self.routing_rules,
                    &self.default_route,
                ) {
                    let branch_response = route_payload
                        .get(fields::BRANCHES)
                        .and_then(|b| b.as_object())
                        .and_then(|obj| obj.get(route_str))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    let route_message = build_route_message(
                        target,
                        route_str,
                        route_payload.get(fields::ROUTE_REASON),
                        branch_response,
                        payload,
                        agent_name,
                    )?;

                    branches.insert(target.clone(), route_message);
                    break;
                }
            }
        }

        Ok(branches)
    }

    /// 创建默认路由
    fn create_default_route(
        &self,
        payload: &Value,
        agent_name: &str,
        default_route: &str,
    ) -> Result<HashMap<String, AgentMessage>> {
        let mut route_message_payload = payload.clone();
        route_message_payload[fields::ROUTE_LABEL] = json!(fields::DEFAULT);
        route_message_payload[fields::ROUTE_REASON] = json!(prompt_consts::DEFAULT_ROUTE_REASON);

        let route_message = build_route_message(
            default_route,
            fields::DEFAULT,
            Some(&route_message_payload[fields::ROUTE_REASON]),
            None,
            payload,
            agent_name,
        )?;

        let mut branches = HashMap::new();
        branches.insert(default_route.to_string(), route_message);

        Ok(branches)
    }
}
