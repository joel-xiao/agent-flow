// Flow 模块 - 工作流定义和执行

pub mod loader;
pub mod constants;
pub mod config;
pub mod agent;
pub mod services;
pub mod types;
pub mod nodes;
pub mod conditions;
pub mod builder;
pub mod registry;

// 重新导出核心类型
pub use types::{Flow, FlowTransition, FlowParameter, FlowParameterKind, FlowVariable};
pub use nodes::{
    FlowNode, FlowNodeKind, DecisionNode, DecisionPolicy, DecisionBranch,
    JoinNode, JoinStrategy, LoopNode, ToolNode,
};
pub use conditions::{
    ConditionFuture, TransitionCondition, LoopContinuationFuture, LoopContinuation,
    condition_from_fn, condition_always, condition_state_equals, condition_state_not_equals,
    condition_state_exists, condition_state_absent,
    loop_condition_from_fn, loop_condition_always,
};
pub use builder::FlowBuilder;
pub use registry::FlowRegistry;
