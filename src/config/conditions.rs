use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 结构化的条件定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    /// 条件类型: state_equals, state_not_equals, state_exists, state_absent, custom
    #[serde(rename = "type")]
    pub condition_type: String,

    /// 条件参数
    #[serde(flatten)]
    pub params: Value,
}
