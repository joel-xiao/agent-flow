/// Agent 驱动类型
/// 
/// 驱动(Driver)仅作为标识符使用，不包含任何业务逻辑或硬编码配置。
/// 所有endpoint、model、api_format等配置都应该从JSON配置文件中读取。
/// 
/// # 设计原则
/// 
/// 1. **纯标识符**: Driver只是一个字符串标识，用于识别提供商
/// 2. **无硬编码**: 不包含默认endpoint、API格式等业务逻辑
/// 3. **配置驱动**: 所有配置从JSON读取
/// 
/// # 配置示例
/// 
/// ```json
/// {
///   "driver": "qwen",  // 仅用于标识和环境变量名
///   "model": "qwen-max",
///   "endpoint": "https://dashscope.aliyuncs.com/compatible-mode/v1",
///   "api_key": "${QWEN_API_KEY}"
/// }
/// ```
/// 
/// # 支持的Driver
/// 
/// - `echo`: 本地回显，用于测试
/// - `qwen`: 通义千问
/// - `moonshot`: 月之暗面
/// - `bigmodel`: 智谱AI
/// - `deepseek`: DeepSeek
/// - `openrouter`: OpenRouter
/// - `doubao`: 豆包
/// - `claude`: Claude
/// - `chatgpt`: ChatGPT/OpenAI
/// - `gemini`: Google Gemini
/// - `mistral`: Mistral AI
/// - `yi`: 零一万物
/// - `generic`: 通用驱动（用于任意兼容OpenAI API的服务）
/// 
/// # 添加新的Driver
/// 
/// 只需在enum中添加新变体，无需添加任何业务逻辑：
/// 
/// ```rust
/// #[cfg(feature = "openai-client")]
/// MyNewLLM,
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentDriverKind {
    Echo,
    #[cfg(feature = "openai-client")]
    Qwen,
    #[cfg(feature = "openai-client")]
    Moonshot,
    #[cfg(feature = "openai-client")]
    BigModel,
    #[cfg(feature = "openai-client")]
    DeepSeek,
    #[cfg(feature = "openai-client")]
    OpenRouter,
    #[cfg(feature = "openai-client")]
    Doubao,
    #[cfg(feature = "openai-client")]
    Claude,
    #[cfg(feature = "openai-client")]
    ChatGPT,
    #[cfg(feature = "openai-client")]
    Gemini,
    #[cfg(feature = "openai-client")]
    Mistral,
    #[cfg(feature = "openai-client")]
    Yi,
    #[cfg(feature = "openai-client")]
    Generic,
}

impl Default for AgentDriverKind {
    fn default() -> Self {
        AgentDriverKind::Echo
    }
}

impl AgentDriverKind {
    /// 获取driver的字符串标识
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentDriverKind::Echo => "echo",
            #[cfg(feature = "openai-client")]
            AgentDriverKind::Qwen => "qwen",
            #[cfg(feature = "openai-client")]
            AgentDriverKind::Moonshot => "moonshot",
            #[cfg(feature = "openai-client")]
            AgentDriverKind::BigModel => "bigmodel",
            #[cfg(feature = "openai-client")]
            AgentDriverKind::DeepSeek => "deepseek",
            #[cfg(feature = "openai-client")]
            AgentDriverKind::OpenRouter => "openrouter",
            #[cfg(feature = "openai-client")]
            AgentDriverKind::Doubao => "doubao",
            #[cfg(feature = "openai-client")]
            AgentDriverKind::Claude => "claude",
            #[cfg(feature = "openai-client")]
            AgentDriverKind::ChatGPT => "chatgpt",
            #[cfg(feature = "openai-client")]
            AgentDriverKind::Gemini => "gemini",
            #[cfg(feature = "openai-client")]
            AgentDriverKind::Mistral => "mistral",
            #[cfg(feature = "openai-client")]
            AgentDriverKind::Yi => "yi",
            #[cfg(feature = "openai-client")]
            AgentDriverKind::Generic => "generic",
        }
    }

    /// 获取默认的环境变量名（仅用于API key）
    /// 
    /// 这是driver唯一保留的"默认值"，因为：
    /// 1. 环境变量名是约定俗成的（如OPENAI_API_KEY）
    /// 2. 不影响业务逻辑
    /// 3. 可以在配置中覆盖
    /// 
    /// # 返回
    /// 
    /// - `Some(&str)`: 该driver对应的环境变量名
    /// - `None`: 该driver没有默认环境变量（如Echo、Generic）
    #[cfg(feature = "openai-client")]
    pub fn default_env_key(&self) -> Option<&'static str> {
        match self {
            AgentDriverKind::Echo => None,
            AgentDriverKind::Qwen => Some("QWEN_API_KEY"),
            AgentDriverKind::Moonshot => Some("MOONSHOT_API_KEY"),
            AgentDriverKind::BigModel => Some("BIGMODEL_API_KEY"),
            AgentDriverKind::DeepSeek => Some("DEEPSEEK_API_KEY"),
            AgentDriverKind::OpenRouter => Some("OPENROUTER_API_KEY"),
            AgentDriverKind::Doubao => Some("DOUBAO_API_KEY"),
            AgentDriverKind::Claude => Some("CLAUDE_API_KEY"),
            AgentDriverKind::ChatGPT => Some("OPENAI_API_KEY"),
            AgentDriverKind::Gemini => Some("GEMINI_API_KEY"),
            AgentDriverKind::Mistral => Some("MISTRAL_API_KEY"),
            AgentDriverKind::Yi => Some("YI_API_KEY"),
            AgentDriverKind::Generic => None,
        }
    }
}

impl<'de> serde::Deserialize<'de> for AgentDriverKind {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "echo" => Ok(AgentDriverKind::Echo),
            #[cfg(feature = "openai-client")]
            "qwen" => Ok(AgentDriverKind::Qwen),
            #[cfg(feature = "openai-client")]
            "moonshot" => Ok(AgentDriverKind::Moonshot),
            #[cfg(feature = "openai-client")]
            "bigmodel" => Ok(AgentDriverKind::BigModel),
            #[cfg(feature = "openai-client")]
            "deepseek" => Ok(AgentDriverKind::DeepSeek),
            #[cfg(feature = "openai-client")]
            "openrouter" => Ok(AgentDriverKind::OpenRouter),
            #[cfg(feature = "openai-client")]
            "doubao" => Ok(AgentDriverKind::Doubao),
            #[cfg(feature = "openai-client")]
            "claude" => Ok(AgentDriverKind::Claude),
            #[cfg(feature = "openai-client")]
            "chatgpt" => Ok(AgentDriverKind::ChatGPT),
            #[cfg(feature = "openai-client")]
            "gemini" => Ok(AgentDriverKind::Gemini),
            #[cfg(feature = "openai-client")]
            "mistral" => Ok(AgentDriverKind::Mistral),
            #[cfg(feature = "openai-client")]
            "yi" => Ok(AgentDriverKind::Yi),
            #[cfg(feature = "openai-client")]
            "generic" => Ok(AgentDriverKind::Generic),
            _ => Err(serde::de::Error::custom(format!("unknown driver: {}", s))),
        }
    }
}
