use super::driver::AgentDriverKind;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

/// Agent 配置
#[derive(Debug, Deserialize, Clone)]
pub struct AgentConfig {
    pub name: String,
    #[serde(default)]
    pub driver: AgentDriverKind,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub intent: Option<String>,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default)]
    pub metadata: Option<Value>,
    /// 路由模式: "auto" 启用自动路由, "manual" 或 None 使用手动路由
    #[serde(default)]
    pub route_mode: Option<String>,
    /// 可路由的目标节点 ID 列表
    #[serde(default)]
    pub route_targets: Option<Vec<String>>,
    /// 路由专用的 prompt, 用于指导 LLM 生成路由标签
    #[serde(default)]
    pub route_prompt: Option<String>,
    /// 默认路由目标（当自动路由失败时使用）
    #[serde(default)]
    pub default_route: Option<String>,
    /// LLM 温度值（默认 0.7）
    #[serde(default)]
    pub temperature: Option<f32>,
    /// 业务规则配置（从 graph_config 读取）
    #[serde(default)]
    pub rules: Option<AgentRulesConfig>,
}

/// Agent 业务规则配置（内部使用）
#[derive(Debug, Deserialize, Clone)]
pub struct AgentRulesConfig {
    /// 字段提取规则
    #[serde(default)]
    pub field_extraction: Option<FieldExtractionRules>,
    /// Prompt 构建规则
    #[serde(default)]
    pub prompt_building: Option<PromptBuildingRules>,
    /// 路由匹配规则
    #[serde(default)]
    pub routing: Option<RoutingRules>,
    /// Payload 构建规则
    #[serde(default)]
    pub payload_building: Option<PayloadBuildingRules>,
}

/// 字段提取规则
#[derive(Debug, Deserialize, Clone)]
pub struct FieldExtractionRules {
    /// 用户输入字段提取优先级（按顺序）
    #[serde(default = "default_user_input_fields")]
    pub user_input_fields: Vec<String>,
    /// Steps 字段名
    #[serde(default = "default_steps_field")]
    pub steps_field: String,
    /// 需要提取并存储到 State 的字段映射 (Response Field -> State Key)
    #[serde(default)]
    pub extract_to_state: Option<HashMap<String, String>>,
}

fn default_user_input_fields() -> Vec<String> {
    vec![
        "response".to_string(),
        "raw".to_string(),
        "user".to_string(),
        "goal".to_string(),
    ]
}

fn default_steps_field() -> String {
    "steps".to_string()
}

/// Prompt 构建规则
#[derive(Debug, Deserialize, Clone)]
pub struct PromptBuildingRules {
    /// Role 模板（支持 {role} 占位符）
    #[serde(default = "default_role_template")]
    pub role_template: String,
    /// Role + Prompt 模板（支持 {role} 和 {prompt} 占位符）
    #[serde(default = "default_role_prompt_template")]
    pub role_prompt_template: String,
    /// 温度值（默认 0.7）
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    /// 历史上下文最大条目数（可选，默认 3）
    #[serde(default)]
    pub max_history_items: Option<usize>,
    /// 需要注入到 Prompt 的 State Store 变量键列表
    #[serde(default)]
    pub include_store_keys: Option<Vec<String>>,
}

fn default_role_template() -> String {
    "You are {}.".to_string()
}

fn default_role_prompt_template() -> String {
    "You are {}. {}".to_string()
}

fn default_temperature() -> f32 {
    0.7
}

/// 路由匹配规则
#[derive(Debug, Deserialize, Clone)]
pub struct RoutingRules {
    /// 路由目标分割符
    #[serde(default = "default_target_separator")]
    pub target_separator: String,
    /// 需要过滤的前缀列表
    #[serde(default = "default_target_prefixes")]
    pub target_prefixes: Vec<String>,
    /// 需要过滤的后缀列表
    #[serde(default = "default_target_suffixes")]
    pub target_suffixes: Vec<String>,
    /// JSON 代码块开始标记
    #[serde(default = "default_json_code_block_start")]
    pub json_code_block_start: String,
    /// 代码块开始标记
    #[serde(default = "default_code_block_start")]
    pub code_block_start: String,
    /// 代码块结束标记
    #[serde(default = "default_code_block_end")]
    pub code_block_end: String,
}

fn default_target_separator() -> String {
    "_".to_string()
}

fn default_target_prefixes() -> Vec<String> {
    vec!["node".to_string()]
}

fn default_target_suffixes() -> Vec<String> {
    vec!["handler".to_string()]
}

fn default_json_code_block_start() -> String {
    "```json".to_string()
}

fn default_code_block_start() -> String {
    "```".to_string()
}

fn default_code_block_end() -> String {
    "```".to_string()
}

/// Payload 构建规则
#[derive(Debug, Deserialize, Clone)]
pub struct PayloadBuildingRules {
    /// 需要添加到 payload 的字段列表
    #[serde(default)]
    pub fields_to_add: Vec<String>,
    /// 图像处理规则
    #[serde(default)]
    pub image_processing: Option<ImageProcessingRules>,
}

/// 图像处理规则
#[derive(Debug, Deserialize, Clone)]
pub struct ImageProcessingRules {
    /// 视觉模型关键词列表
    #[serde(default = "default_vision_keywords")]
    pub vision_keywords: Vec<String>,
}

fn default_vision_keywords() -> Vec<String> {
    vec!["vl".to_string(), "vision".to_string()]
}

/// Tool 驱动类型
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ToolDriverKind {
    Echo,
}

impl Default for ToolDriverKind {
    fn default() -> Self {
        ToolDriverKind::Echo
    }
}

/// Tool 配置
#[derive(Debug, Deserialize, Clone)]
pub struct ToolConfig {
    pub name: String,
    #[serde(default)]
    pub driver: ToolDriverKind,
    #[serde(default)]
    pub description: Option<String>,
}

/// 工作流配置
#[derive(Debug, Deserialize, Clone)]
pub struct WorkflowConfig {
    #[serde(default)]
    pub agents: Vec<AgentConfig>,
    #[serde(default)]
    pub tools: Vec<ToolConfig>,
    pub flow: super::graph::GraphFlow,
}
