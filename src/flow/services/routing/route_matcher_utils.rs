use crate::flow::config::RoutingRules;
use crate::flow::constants::fields;

/// 检查路由标签是否匹配目标节点
pub fn is_route_match(
    route_label: &str,
    target: &str,
    routing_rules: &Option<RoutingRules>,
    default_route: &Option<String>,
) -> bool {
    let target_lower = target.to_lowercase();

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

    let target_route = target_lower
        .split(separator)
        .find(|part| !part.is_empty() && !prefixes.contains(part) && !suffixes.contains(part))
        .unwrap_or("");

    target_lower.contains(route_label)
        || route_label == target_route
        || (route_label == fields::DEFAULT
            && default_route.as_ref().map(|d| d.as_str()) == Some(target))
}
