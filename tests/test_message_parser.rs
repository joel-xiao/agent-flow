// 测试 message_parser 模块

#[cfg(test)]
mod tests {
    use agentflow::flow::services::message_parser::MessageParser;
    use agentflow::agent::{AgentMessage, MessageRole};
    use serde_json::json;

    #[test]
    fn test_parse_payload() {
        let message = AgentMessage {
            id: "test-1".to_string(),
            role: MessageRole::User,
            from: "user".to_string(),
            to: None,
            content: json!({"user": "test"}).to_string(),
            metadata: None,
        };
        
        let history = vec![];
        let result = MessageParser::parse_payload(&message, &history);
        assert!(result.is_ok());
        println!("✅ Parse payload test passed");
    }

    #[test]
    fn test_extract_user_input() {
        let payload = json!({
            "response": "user query"
        });
        
        let history = vec![];
        let result = MessageParser::extract_user_input(&payload, &history, None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "user query");
        println!("✅ Extract user input test passed");
    }
}

