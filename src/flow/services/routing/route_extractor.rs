use crate::flow::config::RoutingRules;
use crate::flow::constants::{fields, prompt as prompt_consts};
use serde_json::{json, Value};

/// 从响应文本中提取路由信息（fallback）
pub fn extract_route_from_text(
    text: &str,
    route_targets: &[String],
    routing_rules: Option<&RoutingRules>,
) -> Option<Value> {
    let lower_text = text.to_lowercase();

    let separator = routing_rules
        .as_ref()
        .map(|r| r.target_separator.as_str())
        .unwrap_or("_");
    let prefixes = routing_rules
        .as_ref()
        .map(|r| {
            r.target_prefixes
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| vec!["node"]);
    let suffixes = routing_rules
        .as_ref()
        .map(|r| {
            r.target_suffixes
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| vec!["handler"]);

    let route_label = route_targets.iter().find_map(|target| {
        let target_parts: Vec<&str> = target.split(separator).collect();
        let target_keyword = target_parts
            .iter()
            .find(|p| !p.is_empty() && !prefixes.contains(p) && !suffixes.contains(p))
            .map(|s| s.to_lowercase());

        if let Some(keyword) = &target_keyword {
            if lower_text.contains(keyword) {
                return Some(keyword.clone());
            }
        }
        None
    });

    route_label.map(|label| {
        json!({
            fields::ROUTE: label,
            fields::ROUTE_REASON: prompt_consts::EXTRACTED_ROUTE_REASON
        })
    })
}
