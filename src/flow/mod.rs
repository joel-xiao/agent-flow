// Flow 模块 - 工作流定义和执行

pub mod agent;
pub mod builder;
pub mod conditions;
pub mod config;
pub mod constants;
pub mod loader;
pub mod nodes;
pub mod registry;
pub mod services;
pub mod types;

// 重新导出核心类型
pub use builder::FlowBuilder;
pub use conditions::{
    condition_always, condition_from_fn, condition_state_absent, condition_state_equals,
    condition_state_exists, condition_state_not_equals, loop_condition_always,
    loop_condition_from_fn, ConditionFuture, LoopContinuation, LoopContinuationFuture,
    TransitionCondition,
};
pub use nodes::{
    DecisionBranch, DecisionNode, DecisionPolicy, FlowNode, FlowNodeKind, JoinNode, JoinStrategy,
    LoopNode, ToolNode,
};
pub use registry::FlowRegistry;
pub use types::{Flow, FlowParameter, FlowParameterKind, FlowTransition, FlowVariable};
