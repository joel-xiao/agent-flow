use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Agent 业务规则配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRules {
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadBuildingRules {
    /// 需要添加到 payload 的字段列表
    #[serde(default)]
    pub fields_to_add: Vec<String>,
    /// 图像处理规则
    #[serde(default)]
    pub image_processing: Option<ImageProcessingRules>,
}

/// 图像处理规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageProcessingRules {
    /// 视觉模型关键词列表
    #[serde(default = "default_vision_keywords")]
    pub vision_keywords: Vec<String>,
}

fn default_vision_keywords() -> Vec<String> {
    vec!["vl".to_string(), "vision".to_string()]
}
