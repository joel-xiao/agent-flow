// 路由服务模块

mod matcher;
mod message_builder;
mod route_extractor;
mod route_matcher_utils;
mod response_cleaner;

pub use matcher::RouteMatcher;
pub use response_cleaner::clean_response;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use route_matcher_utils::is_route_match;
    use route_extractor::extract_route_from_text;

    #[test]
    fn test_is_route_match() {
        assert!(is_route_match("urgent", "node_urgent_handler", &None, &None));
        assert!(!is_route_match("normal", "node_urgent_handler", &None, &None));
    }
    
    #[test]
    fn test_clean_response() {
        let response = r#"```json
{"route": "urgent"}
```"#;
        
        let cleaned = clean_response(response, None);
        assert!(cleaned.contains("urgent"));
        assert!(!cleaned.contains("```"));
    }
    
    #[test]
    fn test_extract_route_from_text() {
        let route_targets = vec!["node_urgent_handler".to_string()];
        let text = "This is an urgent request";
        
        let route = extract_route_from_text(text, &route_targets, None);
        assert!(route.is_some());
        assert_eq!(route.unwrap()["route"], "urgent");
    }
}

