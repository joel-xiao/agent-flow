use crate::flow::conditions::{LoopContinuation, TransitionCondition};
use serde_json::Value;

/// Flow 节点类型定义

/// Flow 节点
#[derive(Clone, Debug)]
pub struct FlowNode {
    pub name: String,
    pub kind: FlowNodeKind,
    pub metadata: Option<Value>,
}

/// Flow 节点类型
#[derive(Clone, Debug)]
pub enum FlowNodeKind {
    Agent(String),
    Terminal,
    Decision(DecisionNode),
    Join(JoinNode),
    Loop(LoopNode),
    Tool(ToolNode),
}

/// 决策节点
#[derive(Clone)]
pub struct DecisionNode {
    pub policy: DecisionPolicy,
    pub branches: Vec<DecisionBranch>,
}

/// 决策策略
#[derive(Clone, Debug)]
pub enum DecisionPolicy {
    FirstMatch,
    AllMatches,
}

/// 决策分支
#[derive(Clone)]
pub struct DecisionBranch {
    pub name: Option<String>,
    pub condition: Option<TransitionCondition>,
    pub target: String,
}

/// 合并节点
#[derive(Clone, Debug)]
pub struct JoinNode {
    pub strategy: JoinStrategy,
    pub inbound: Vec<String>,
}

/// 合并策略
#[derive(Clone, Debug)]
pub enum JoinStrategy {
    All,
    Any,
    Count(usize),
}

/// 循环节点
#[derive(Clone)]
pub struct LoopNode {
    pub entry: String,
    pub condition: Option<LoopContinuation>,
    pub max_iterations: Option<u32>,
    pub exit: Option<String>,
}

/// 工具节点
#[derive(Clone, Debug)]
pub struct ToolNode {
    pub pipeline: String,
    pub params: Option<serde_json::Value>,
}

use std::fmt;

impl fmt::Debug for DecisionNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DecisionNode")
            .field("policy", &self.policy)
            .field("branches", &self.branches)
            .finish()
    }
}

impl fmt::Debug for DecisionBranch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DecisionBranch")
            .field("name", &self.name)
            .field("target", &self.target)
            .field("has_condition", &self.condition.as_ref().map(|_| true))
            .finish()
    }
}

impl fmt::Debug for LoopNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LoopNode")
            .field("entry", &self.entry)
            .field("max_iterations", &self.max_iterations)
            .field("exit", &self.exit)
            .field("has_condition", &self.condition.as_ref().map(|_| true))
            .finish()
    }
}
