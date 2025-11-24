use crate::agent::AgentMessage;

/// 运行时类型定义

/// Flow 执行事件
#[derive(Clone)]
pub struct FlowEvent {
    pub node: String,
    pub message: AgentMessage,
    pub iterations: u32,
    pub trace_id: String,
    pub source: String,
}

/// 任务执行结果
pub enum TaskResult {
    Continue,
    Finished(TaskFinished),
}

/// 任务完成信息
pub struct TaskFinished {
    pub node: String,
    pub message: Option<AgentMessage>,
}

/// Flow 执行结果
pub struct FlowExecution {
    pub flow_name: String,
    pub last_node: String,
    pub last_message: Option<AgentMessage>,
    pub errors: Vec<crate::error::FrameworkError>,
}
