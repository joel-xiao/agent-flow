// 编译检查测试 - 用于排查编译卡死问题

#[test]
fn test_imports() {
    // 测试关键模块是否能正常导入
    use agentflow::flow::services::prompt_builder::PromptBuilder;
    use agentflow::flow::services::message_parser::MessageParser;
    use agentflow::flow::services::llm_caller::LlmCaller;
    use agentflow::agent::{AgentMessage, MessageRole};
    use agentflow::state::FlowContext;
    
    // 测试 AgentMessage 结构
    let _msg = AgentMessage {
        id: "test".to_string(),
        role: MessageRole::User,
        from: "test".to_string(),
        to: None,
        content: "test".to_string(),
        metadata: None,
    };
    
    println!("✅ All imports successful");
}

#[test]
fn test_constants() {
    // 测试常量是否能正常访问
    use agentflow::flow::constants::prompt as prompt_consts;
    
    let template = prompt_consts::TEMPLATE_ROLE_AND_PROMPT;
    assert!(!template.is_empty());
    println!("✅ Constants access successful: {}", template);
}

#[test]
fn test_string_replace() {
    // 测试字符串替换功能
    let template = "You are {}. {}";
    let result = template.replace("{}", "test");
    assert_eq!(result, "You are test. test");
    println!("✅ String replace test passed");
}

