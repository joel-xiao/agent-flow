/// 流程相关的常量定义
/// 
/// 统一管理所有硬编码的字符串常量、魔法值等

/// Payload 字段名常量
pub mod fields {
    pub const STEPS: &str = "steps";
    pub const RESPONSE: &str = "response";
    pub const RAW: &str = "raw";
    pub const USER: &str = "user";
    pub const GOAL: &str = "goal";
    pub const LAST_AGENT: &str = "last_agent";
    pub const PROMPT: &str = "prompt";
    pub const ROLE: &str = "role";
    pub const MODEL: &str = "model";
    pub const AGENT_METADATA: &str = "agent_metadata";
    pub const INTENT: &str = "intent";
    pub const DRIVER: &str = "driver";
    pub const AGENT: &str = "agent";
    
    // 路由相关字段
    pub const ROUTE: &str = "route";
    pub const ROUTE_LABEL: &str = "route_label";
    pub const ROUTE_REASON: &str = "route_reason";
    pub const ROUTE_MODE: &str = "route_mode";
    pub const ROUTE_TARGETS: &str = "route_targets";
    pub const ROUTE_PROMPT: &str = "route_prompt";
    pub const DEFAULT_ROUTE: &str = "default_route";
    pub const DEFAULT: &str = "default";
    pub const BRANCHES: &str = "branches";
    
    // 图像相关字段
    pub const IMAGE_URL: &str = "image_url";
    pub const IMAGE_BASE64: &str = "image_base64";
    pub const IMAGE_PATH: &str = "image_path";
}

/// LLM 配置常量
pub mod llm {
    /// 默认温度值
    pub const DEFAULT_TEMPERATURE: f32 = 0.7;
    
    /// 视觉模型关键词
    pub const VISION_KEYWORD_VL: &str = "vl";
    pub const VISION_KEYWORD_VISION: &str = "vision";
}

/// 路由相关常量
pub mod routing {
    /// 自动路由模式
    pub const MODE_AUTO: &str = "auto";
    
    /// 手动路由模式
    pub const MODE_MANUAL: &str = "manual";
    
    /// 路由目标分割时跳过的前缀/后缀
    pub const TARGET_PREFIX_NODE: &str = "node";
    pub const TARGET_SUFFIX_HANDLER: &str = "handler";
    
    /// JSON 代码块标记
    pub const JSON_CODE_BLOCK_START: &str = "```json";
    pub const CODE_BLOCK_START: &str = "```";
    pub const CODE_BLOCK_END: &str = "```";
}

/// Agent 配置常量
pub mod agent {
    /// Qwen 视觉模型的默认端点（如果配置中未提供时使用）
    pub const DEFAULT_QWEN_VISION_ENDPOINT: &str = 
        "https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions";
}

/// 系统提示词模板常量
pub mod prompt {
    pub const TEMPLATE_ROLE_AND_PROMPT: &str = "You are {}. {}";
    pub const TEMPLATE_ROLE_ONLY: &str = "You are {}.";
    
    pub const ROUTING_INSTRUCTION_PREFIX: &str = 
        "\n\nIMPORTANT: You must analyze the request and determine the appropriate route. ";
    
    pub const ROUTING_INSTRUCTION_AVAILABLE_ROUTES: &str = 
        "Available routes: {}. ";
    
    pub const ROUTING_INSTRUCTION_JSON_FORMAT: &str = 
        "You MUST respond with valid JSON format: {{\"route\": \"<route_label>\", \"response\": \"<your_analysis>\", \"reason\": \"<routing_reason>\"}}";
    
    pub const ROUTING_INSTRUCTION_AVAILABLE_TARGETS: &str = 
        "Available route targets: {}. ";
    
    pub const ROUTING_INSTRUCTION_JSON_FORMAT_TARGETS: &str = 
        "You MUST respond with valid JSON format: {{\"route\": \"<target_name_or_label>\", \"response\": \"<your_analysis>\", \"reason\": \"<routing_reason>\"}}";
    
    pub const DEFAULT_ROUTE_REASON: &str = "No matching route found, using default";
    pub const EXTRACTED_ROUTE_REASON: &str = "Extracted from response text";
}

