use thiserror::Error;

pub type Result<T> = std::result::Result<T, AgentFlowError>;

#[derive(Debug, Error)]
pub enum AgentFlowError {
    #[error("unknown node `{0}` in flow")]
    UnknownNode(String),
    #[error("agent `{0}` not registered")]
    AgentNotRegistered(String),
    #[error("tool `{0}` not registered")]
    ToolNotRegistered(String),
    #[error("flow `{0}` not registered")]
    FlowNotRegistered(String),
    #[error("invalid transition from `{from}` to `{to}`")]
    InvalidTransition { from: String, to: String },
    #[error("maximum iterations {0} exceeded")]
    MaxIterationsExceeded(u32),
    #[error("context error: {0}")]
    Context(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
