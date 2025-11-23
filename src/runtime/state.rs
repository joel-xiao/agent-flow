use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::agent::{AgentMessage, MessageRole};
use crate::flow::{JoinNode, JoinStrategy};

/// 运行时状态管理

/// 共享状态
#[derive(Default)]
pub struct SharedState {
    pub join_states: Mutex<HashMap<String, JoinState>>,
    pub loop_states: Mutex<HashMap<String, LoopState>>,
    pub started_agents: Mutex<HashSet<String>>,
}

/// Join 节点状态
pub struct JoinState {
    strategy: JoinStrategy,
    pub expected: HashSet<String>,
    received: HashMap<String, AgentMessage>,
    triggered: bool,
}

impl JoinState {
    pub fn new(node: JoinNode) -> Self {
        Self {
            strategy: node.strategy,
            expected: node.inbound.into_iter().collect(),
            received: HashMap::new(),
            triggered: false,
        }
    }

    pub fn record(
        &mut self,
        source: String,
        message: AgentMessage,
    ) -> Option<HashMap<String, AgentMessage>> {
        if self.triggered {
            return None;
        }

        self.received.insert(source.clone(), message);

        match &self.strategy {
            JoinStrategy::All => {
                let required = if self.expected.is_empty() {
                    !self.received.is_empty()
                } else {
                    self.expected
                        .iter()
                        .all(|name| self.received.contains_key(name))
                };
                if required {
                    self.triggered = true;
                    return Some(self.received.clone());
                }
            }
            JoinStrategy::Any => {
                self.triggered = true;
                if let Some(message) = self.received.get(&source).cloned() {
                    let mut map = HashMap::new();
                    map.insert(source, message);
                    return Some(map);
                }
            }
            JoinStrategy::Count(count) => {
                if self.received.len() >= *count {
                    self.triggered = true;
                    return Some(self.received.clone());
                }
            }
        }

        None
    }
}

/// Loop 节点状态
#[derive(Default)]
pub struct LoopState {
    pub iterations: u32,
}

/// 创建 Join 消息
pub fn make_join_message(node_name: &str, messages: &HashMap<String, AgentMessage>) -> AgentMessage {
    let aggregated: Vec<_> = messages
        .iter()
        .map(|(source, message)| {
            serde_json::json!({
                "source": source,
                "id": message.id.clone(),
                "role": format!("{:?}", message.role),
                "content": message.content.clone(),
                "metadata": message.metadata.clone(),
            })
        })
        .collect();

    let payload = serde_json::json!({
        "join_node": node_name,
        "messages": aggregated,
    });

    AgentMessage {
        id: crate::agent::message::uuid(),
        role: crate::agent::MessageRole::System,
        from: node_name.to_string(),
        to: None,
        content: payload.to_string(),
        metadata: Some(payload),
    }
}

