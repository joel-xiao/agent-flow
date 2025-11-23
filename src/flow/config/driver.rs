// use serde::Deserialize;

/// Agent 驱动类型
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

    /// 获取默认的端点地址
    /// 
    /// 返回该 driver 的默认端点地址。
    /// 如果配置中提供了 endpoint，则优先使用配置的值；否则使用此默认值。
    #[cfg(feature = "openai-client")]
    pub fn default_endpoint(&self) -> Option<&'static str> {
        match self {
            AgentDriverKind::Echo => None,
            AgentDriverKind::Qwen => Some("https://dashscope.aliyuncs.com/api/v1/services/aigc/text-generation/generation"),
            AgentDriverKind::Moonshot => Some("https://api.moonshot.cn/v1/chat/completions"),
            AgentDriverKind::BigModel => Some("https://open.bigmodel.cn/api/paas/v4/chat/completions"),
            AgentDriverKind::DeepSeek => Some("https://api.deepseek.com/chat/completions"),
            AgentDriverKind::OpenRouter => Some("https://openrouter.ai/api/v1/chat/completions"),
            AgentDriverKind::Doubao => Some("https://ark.cn-beijing.volces.com/api/v3/chat/completions"),
            AgentDriverKind::Claude => Some("https://api.anthropic.com/v1/messages"),
            AgentDriverKind::ChatGPT => Some("https://api.openai.com/v1/chat/completions"),
            AgentDriverKind::Gemini => Some("https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash-latest:generateContent"),
            AgentDriverKind::Mistral => Some("https://api.mistral.ai/v1/chat/completions"),
            AgentDriverKind::Yi => Some("https://api.lingyiwanwu.com/v1/chat/completions"),
            AgentDriverKind::Generic => None,
        }
    }

    #[cfg(feature = "openai-client")]
    pub fn api_format(&self) -> Option<crate::llm::ApiFormat> {
        match self {
            AgentDriverKind::Echo => None,
            AgentDriverKind::Qwen => Some(crate::llm::ApiFormat::Qwen),
            AgentDriverKind::Moonshot => Some(crate::llm::ApiFormat::OpenAI),
            AgentDriverKind::BigModel => Some(crate::llm::ApiFormat::OpenAI),
            AgentDriverKind::DeepSeek => Some(crate::llm::ApiFormat::OpenAI),
            AgentDriverKind::OpenRouter => Some(crate::llm::ApiFormat::OpenAI),
            AgentDriverKind::Doubao => Some(crate::llm::ApiFormat::OpenAI),
            AgentDriverKind::Claude => Some(crate::llm::ApiFormat::OpenAI),
            AgentDriverKind::ChatGPT => Some(crate::llm::ApiFormat::OpenAI),
            AgentDriverKind::Gemini => Some(crate::llm::ApiFormat::OpenAI),
            AgentDriverKind::Mistral => Some(crate::llm::ApiFormat::OpenAI),
            AgentDriverKind::Yi => Some(crate::llm::ApiFormat::OpenAI),
            AgentDriverKind::Generic => None,
        }
    }

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

