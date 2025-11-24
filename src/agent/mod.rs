pub mod agent;
pub mod builtin;
pub mod factory;
pub mod manifest;
pub mod message;
pub mod registry;

pub use agent::{Agent, AgentAction, AgentContext, AgentInput, AgentOutput, AgentRuntime};
pub use factory::{AgentFactory, AgentFactoryRegistry};
pub use manifest::{AgentManifest, AgentManifestBuilder, AgentPort, AgentPortSchema};
pub use message::{AgentMessage, MessageRole};
pub use registry::{register_agent, AgentRegistry};

// Re-export uuid for backward compatibility
pub use message::uuid;
