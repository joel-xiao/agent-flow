use crate::flow::config::RoutingRules;

/// 清理响应内容，提取 JSON（处理代码块包裹的情况）
pub fn clean_response(response: &str, routing_rules: Option<&RoutingRules>) -> String {
    let json_code_block_start = routing_rules
        .as_ref()
        .map(|r| r.json_code_block_start.as_str())
        .unwrap_or("```json");
    let code_block_start = routing_rules
        .as_ref()
        .map(|r| r.code_block_start.as_str())
        .unwrap_or("```");
    let code_block_end = routing_rules
        .as_ref()
        .map(|r| r.code_block_end.as_str())
        .unwrap_or("```");

    if response.contains(json_code_block_start) {
        if let Some(start) = response.find(json_code_block_start) {
            let json_start = start + json_code_block_start.len();
            if let Some(end) = response[json_start..].find(code_block_end) {
                let json_end = json_start + end;
                return response[json_start..json_end].trim().to_string();
            }
        }
    } else if response.contains(code_block_start) {
        if let Some(start) = response.find(code_block_start) {
            let json_start = start + code_block_start.len();
            if let Some(end) = response[json_start..].find(code_block_end) {
                let json_end = json_start + end;
                return response[json_start..json_end].trim().to_string();
            }
        }
    }

    response.to_string()
}
