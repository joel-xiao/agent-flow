// 测试 prompt_builder 模块，排查编译卡死问题

#[cfg(test)]
mod tests {
    use agentflow::flow::services::prompt_builder::PromptBuilder;

    #[test]
    fn test_build_system_prompt_basic() {
        // 测试基本的 prompt 构建
        let result = PromptBuilder::build_system_prompt(
            Some("assistant"),
            Some("You are helpful"),
            None,
        );
        assert!(result.is_ok());
        let prompt = result.unwrap();
        assert!(prompt.contains("assistant"));
        assert!(prompt.contains("helpful"));
        println!("✅ Basic prompt test passed: {}", prompt);
    }

    #[test]
    fn test_build_system_prompt_role_only() {
        // 测试只有 role 的情况
        let result = PromptBuilder::build_system_prompt(
            Some("assistant"),
            None,
            None,
        );
        assert!(result.is_ok());
        let prompt = result.unwrap();
        assert!(prompt.contains("assistant"));
        println!("✅ Role-only prompt test passed: {}", prompt);
    }

    #[test]
    fn test_build_system_prompt_with_routing() {
        // 测试带路由的 prompt 构建
        let route_targets = vec!["node_food_identify".to_string(), "node_portion_analysis".to_string()];
        let result = PromptBuilder::build_system_prompt_with_routing(
            Some("assistant"),
            Some("You are helpful"),
            Some("auto"),
            Some(&route_targets),
            None,
            None,
        );
        assert!(result.is_ok());
        let prompt = result.unwrap();
        assert!(prompt.contains("assistant"));
        println!("✅ Routing prompt test passed");
    }

    #[test]
    fn test_template_replacement() {
        // 测试模板替换功能 - 验证 replace 方法
        let template = "You are {}. {}";
        let result = template.replace("{}", "test");
        assert!(result.contains("test"));
        println!("✅ Template replacement test passed");
    }
}
