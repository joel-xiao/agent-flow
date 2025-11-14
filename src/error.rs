use serde::{Deserialize, Serialize};
use serde_json::Value;
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
    #[error("loop `{node}` exceeded maximum iterations {max}")]
    LoopBoundExceeded { node: String, max: u32 },
    #[error("decision node `{node}` had no matching branches")]
    DecisionNoMatch { node: String },
    #[error("join node `{node}` did not receive required inbound branches")]
    JoinIncomplete { node: String },
    #[error("message serialization error: {0}")]
    Serialization(String),
    #[error("{kind} manifest mismatch for `{name}`")]
    ManifestMismatch { kind: &'static str, name: String },
    #[error("context error: {0}")]
    Context(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ErrorSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FrameworkError {
    pub code: String,
    pub message: String,
    #[serde(default = "FrameworkError::default_severity")]
    pub severity: ErrorSeverity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

impl FrameworkError {
    fn default_severity() -> ErrorSeverity {
        ErrorSeverity::Error
    }

    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            severity: Self::default_severity(),
            context: None,
            source: None,
        }
    }

    pub fn with_severity(mut self, severity: ErrorSeverity) -> Self {
        self.severity = severity;
        self
    }

    pub fn with_context(mut self, context: Value) -> Self {
        self.context = Some(context);
        self
    }

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }
}

impl From<AgentFlowError> for FrameworkError {
    fn from(error: AgentFlowError) -> Self {
        match error {
            AgentFlowError::UnknownNode(name) => FrameworkError::new(
                "flow.unknown_node",
                format!("unknown node `{name}` in flow"),
            ),
            AgentFlowError::AgentNotRegistered(name) => FrameworkError::new(
                "agent.not_registered",
                format!("agent `{name}` not registered"),
            ),
            AgentFlowError::ToolNotRegistered(name) => FrameworkError::new(
                "tool.not_registered",
                format!("tool `{name}` not registered"),
            ),
            AgentFlowError::FlowNotRegistered(name) => FrameworkError::new(
                "flow.not_registered",
                format!("flow `{name}` not registered"),
            ),
            AgentFlowError::InvalidTransition { from, to } => FrameworkError::new(
                "flow.invalid_transition",
                format!("invalid transition from `{from}` to `{to}`"),
            ),
            AgentFlowError::MaxIterationsExceeded(limit) => FrameworkError::new(
                "flow.max_iterations_exceeded",
                format!("maximum iterations {limit} exceeded"),
            )
            .with_severity(ErrorSeverity::Warning),
            AgentFlowError::LoopBoundExceeded { node, max } => FrameworkError::new(
                "flow.loop_max_iterations",
                format!("loop `{node}` exceeded maximum iterations {max}"),
            )
            .with_severity(ErrorSeverity::Warning),
            AgentFlowError::DecisionNoMatch { node } => FrameworkError::new(
                "flow.decision_no_match",
                format!("decision node `{node}` had no matching branches"),
            ),
            AgentFlowError::JoinIncomplete { node } => FrameworkError::new(
                "flow.join_incomplete",
                format!("join node `{node}` did not receive required inbound branches"),
            ),
            AgentFlowError::Serialization(message) => {
                FrameworkError::new("message.serialization_error", message)
            }
            AgentFlowError::ManifestMismatch { kind, name } => FrameworkError::new(
                "manifest.mismatch",
                format!("{kind} manifest mismatch: `{name}`"),
            )
            .with_severity(ErrorSeverity::Error),
            AgentFlowError::Context(message) => FrameworkError::new("context.error", message),
            AgentFlowError::Other(other) => {
                FrameworkError::new("internal.error", other.to_string())
            }
        }
    }
}
