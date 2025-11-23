use serde::Deserialize;
use serde_json::Value;
use crate::flow::{
    FlowParameter, FlowParameterKind, FlowVariable,
    TransitionCondition, LoopContinuation,
    condition_always, condition_state_absent, condition_state_equals,
    condition_state_exists, condition_state_not_equals, loop_condition_always,
};
use crate::state::FlowScopeKind;

/// Graph 工作流参数配置
#[derive(Debug, Deserialize, Clone)]
pub struct GraphParameter {
    pub name: String,
    #[serde(default = "GraphParameter::default_kind")]
    pub kind: String,
    #[serde(default)]
    pub type_name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

impl GraphParameter {
    fn default_kind() -> String {
        "input".into()
    }

    pub fn into_flow_param(self) -> FlowParameter {
        let mut param = match self.kind.as_str() {
            "input" => FlowParameter::new(self.name.clone(), FlowParameterKind::Input),
            "output" => FlowParameter::new(self.name.clone(), FlowParameterKind::Output),
            "inout" => FlowParameter::new(self.name.clone(), FlowParameterKind::InOut),
            _ => FlowParameter::new(self.name.clone(), FlowParameterKind::Input),
        };
        if let Some(type_name) = self.type_name {
            param = param.with_type(type_name);
        }
        if let Some(desc) = self.description {
            param = param.with_description(desc);
        }
        param
    }
}

/// Graph 工作流变量配置
#[derive(Debug, Deserialize, Clone)]
pub struct GraphVariable {
    pub name: String,
    #[serde(default = "GraphVariable::default_scope")]
    pub scope: String,
    #[serde(default)]
    pub default: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

impl GraphVariable {
    fn default_scope() -> String {
        "global".into()
    }

    pub fn into_flow_variable(self) -> FlowVariable {
        let scope = match self.scope.as_str() {
            "global" => FlowScopeKind::Global,
            other => FlowScopeKind::Node(other.into()),
        };
        let mut variable = FlowVariable::new(self.name, scope);
        if let Some(default) = self.default {
            variable = variable.with_default(default);
        }
        if let Some(description) = self.description {
            variable = variable.with_description(description);
        }
        variable
    }
}

/// Graph 工作流转换配置
#[derive(Debug, Deserialize, Clone)]
pub struct GraphTransition {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub condition: Option<GraphCondition>,
}

/// Graph 条件配置
#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GraphCondition {
    Always,
    StateEquals { key: String, value: String },
    StateNotEquals { key: String, value: String },
    StateExists { key: String },
    StateAbsent { key: String },
}

impl GraphCondition {
    pub fn build(&self) -> TransitionCondition {
        match self {
            GraphCondition::Always => condition_always(),
            GraphCondition::StateEquals { key, value } => {
                condition_state_equals(key.clone(), value.clone())
            }
            GraphCondition::StateNotEquals { key, value } => {
                condition_state_not_equals(key.clone(), value.clone())
            }
            GraphCondition::StateExists { key } => condition_state_exists(key.clone()),
            GraphCondition::StateAbsent { key } => condition_state_absent(key.clone()),
        }
    }
}

/// Graph 循环条件配置
#[derive(Debug, Deserialize, Clone)]
pub struct GraphLoopCondition {
    #[serde(default)]
    pub state_equals: Option<LoopConditionStateEquals>,
}

/// 循环条件状态等于配置
#[derive(Debug, Deserialize, Clone)]
pub struct LoopConditionStateEquals {
    pub key: String,
    pub value: String,
}

impl GraphLoopCondition {
    pub fn build(&self) -> LoopContinuation {
        if let Some(state) = &self.state_equals {
            let key = state.key.clone();
            let value = state.value.clone();
            Arc::new(move |ctx| {
                let store = ctx.store();
                let key = key.clone();
                let value = value.clone();
                Box::pin(async move {
                    match store.get(&key).await {
                        Ok(Some(current)) => current == value,
                        _ => false,
                    }
                })
            })
        } else {
            loop_condition_always()
        }
    }
}

use std::sync::Arc;

/// Graph 节点配置
#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GraphNode {
    Agent {
        name: String,
        agent: String,
    },
    Decision {
        name: String,
        policy: Option<String>,
        branches: Vec<GraphDecisionBranch>,
    },
    Join {
        name: String,
        strategy: String,
        inbound: Vec<String>,
    },
    Loop {
        name: String,
        entry: String,
        #[serde(default)]
        condition: Option<GraphLoopCondition>,
        #[serde(default)]
        max_iterations: Option<u32>,
        #[serde(default)]
        exit: Option<String>,
    },
    Tool {
        name: String,
        pipeline: String,
    },
    Terminal {
        name: String,
    },
}

/// Graph 决策分支配置
#[derive(Debug, Deserialize, Clone)]
pub struct GraphDecisionBranch {
    pub target: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub condition: Option<GraphCondition>,
}

/// Graph 工作流配置
#[derive(Debug, Deserialize, Clone)]
pub struct GraphFlow {
    pub name: String,
    pub start: String,
    #[serde(default)]
    pub parameters: Vec<GraphParameter>,
    #[serde(default)]
    pub variables: Vec<GraphVariable>,
    #[serde(default)]
    pub nodes: Vec<GraphNode>,
    #[serde(default)]
    pub transitions: Vec<GraphTransition>,
}

