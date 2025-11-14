use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use async_trait::async_trait;
use parking_lot::RwLock;

use crate::agent::AgentMessage;
use crate::error::{AgentFlowError, Result};

#[async_trait]
pub trait ContextStore: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<String>>;
    async fn set(&self, key: &str, value: String) -> Result<()>;
    async fn delete(&self, key: &str) -> Result<()>;
}

#[derive(Clone)]
pub struct FlowContext {
    store: Arc<dyn ContextStore>,
    messages: Arc<RwLock<Vec<AgentMessage>>>,
    scopes: Arc<ScopeStack>,
    global_scope_id: ScopeId,
}

impl FlowContext {
    pub fn new(store: Arc<dyn ContextStore>) -> Self {
        let scopes = Arc::new(ScopeStack::default());
        let global_scope_id = scopes.push_scope(FlowScopeKind::Global);
        Self {
            store,
            messages: Arc::new(RwLock::new(Vec::new())),
            scopes,
            global_scope_id,
        }
    }

    pub fn store(&self) -> Arc<dyn ContextStore> {
        Arc::clone(&self.store)
    }

    pub fn push_message(&self, message: AgentMessage) {
        self.messages.write().push(message);
    }

    pub fn history(&self) -> Vec<AgentMessage> {
        self.messages.read().clone()
    }

    pub fn last_message(&self) -> Option<AgentMessage> {
        self.messages.read().last().cloned()
    }

    pub fn clear_messages(&self) {
        self.messages.write().clear();
    }

    pub fn session(&self) -> SessionContext {
        SessionContext {
            store: Arc::clone(&self.store),
        }
    }

    pub fn scope(&self, kind: FlowScopeKind) -> FlowScopeGuard {
        let scope_id = self.scopes.push_scope(kind.clone());
        FlowScopeGuard {
            stack: Arc::clone(&self.scopes),
            id: scope_id,
            kind,
        }
    }

    pub fn variables(&self) -> FlowVariables {
        FlowVariables {
            stack: Arc::clone(&self.scopes),
            global_scope_id: self.global_scope_id,
        }
    }
}

pub struct MemoryStore {
    inner: RwLock<HashMap<String, String>>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl ContextStore for MemoryStore {
    async fn get(&self, key: &str) -> Result<Option<String>> {
        Ok(self.inner.read().get(key).cloned())
    }

    async fn set(&self, key: &str, value: String) -> Result<()> {
        self.inner.write().insert(key.to_string(), value);
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        self.inner.write().remove(key);
        Ok(())
    }
}

#[cfg(feature = "redis-store")]
pub mod redis {
    use super::*;
    use crate::error::AgentFlowError;
    use redis::AsyncCommands;

    pub struct RedisStore {
        client: redis::Client,
    }

    impl RedisStore {
        pub fn new(client: redis::Client) -> Self {
            Self { client }
        }
    }

    #[async_trait]
    impl ContextStore for RedisStore {
        async fn get(&self, key: &str) -> Result<Option<String>> {
            let mut conn = self
                .client
                .get_multiplexed_async_connection()
                .await
                .map_err(|e| AgentFlowError::Context(e.to_string()))?;
            let value: Option<String> = conn
                .get(key)
                .await
                .map_err(|e| AgentFlowError::Context(e.to_string()))?;
            Ok(value)
        }

        async fn set(&self, key: &str, value: String) -> Result<()> {
            let mut conn = self
                .client
                .get_multiplexed_async_connection()
                .await
                .map_err(|e| AgentFlowError::Context(e.to_string()))?;
            conn.set(key, value)
                .await
                .map_err(|e| AgentFlowError::Context(e.to_string()))?;
            Ok(())
        }

        async fn delete(&self, key: &str) -> Result<()> {
            let mut conn = self
                .client
                .get_multiplexed_async_connection()
                .await
                .map_err(|e| AgentFlowError::Context(e.to_string()))?;
            conn.del(key)
                .await
                .map_err(|e| AgentFlowError::Context(e.to_string()))?;
            Ok(())
        }
    }
}

const SESSION_PREFIX: &str = "session";

#[derive(Clone)]
pub struct SessionContext {
    store: Arc<dyn ContextStore>,
}

impl SessionContext {
    fn key_with_prefix(key: &str) -> String {
        format!("{SESSION_PREFIX}:{key}")
    }

    pub async fn get(&self, key: &str) -> Result<Option<String>> {
        self.store.get(&Self::key_with_prefix(key)).await
    }

    pub async fn set(&self, key: &str, value: impl Into<String>) -> Result<()> {
        self.store
            .set(&Self::key_with_prefix(key), value.into())
            .await
    }

    pub async fn delete(&self, key: &str) -> Result<()> {
        self.store.delete(&Self::key_with_prefix(key)).await
    }
}

type ScopeId = u64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FlowScopeKind {
    Global,
    Node(String),
    Branch(String),
    Custom(String),
}

impl FlowScopeKind {
    fn as_str(&self) -> &str {
        match self {
            FlowScopeKind::Global => "global",
            FlowScopeKind::Node(_) => "node",
            FlowScopeKind::Branch(_) => "branch",
            FlowScopeKind::Custom(_) => "custom",
        }
    }
}

#[derive(Default)]
struct ScopeStack {
    frames: RwLock<Vec<ScopeFrame>>,
}

impl ScopeStack {
    fn push_scope(&self, kind: FlowScopeKind) -> ScopeId {
        let id = NEXT_SCOPE_ID.fetch_add(1, Ordering::Relaxed);
        self.frames.write().push(ScopeFrame::new(id, kind));
        id
    }

    fn remove(&self, id: ScopeId) {
        let mut frames = self.frames.write();
        if let Some(pos) = frames.iter().rposition(|frame| frame.id == id) {
            frames.remove(pos);
        }
    }

    fn with_frame_mut<R>(
        &self,
        id: ScopeId,
        apply: impl FnOnce(&mut ScopeFrame) -> R,
    ) -> Option<R> {
        let mut frames = self.frames.write();
        frames.iter_mut().find(|frame| frame.id == id).map(apply)
    }

    fn with_frames<R>(&self, apply: impl FnOnce(&[ScopeFrame]) -> R) -> R {
        let frames = self.frames.read();
        apply(&frames)
    }
}

#[derive(Clone)]
struct ScopeFrame {
    id: ScopeId,
    kind: FlowScopeKind,
    variables: HashMap<String, String>,
}

impl ScopeFrame {
    fn new(id: ScopeId, kind: FlowScopeKind) -> Self {
        Self {
            id,
            kind,
            variables: HashMap::new(),
        }
    }
}

impl fmt::Debug for ScopeFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScopeFrame")
            .field("id", &self.id)
            .field("kind", &self.kind)
            .field("keys", &self.variables.keys().collect::<Vec<_>>())
            .finish()
    }
}

static NEXT_SCOPE_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
pub struct FlowScopeGuard {
    stack: Arc<ScopeStack>,
    id: ScopeId,
    kind: FlowScopeKind,
}

impl FlowScopeGuard {
    pub fn kind(&self) -> &FlowScopeKind {
        &self.kind
    }

    pub async fn set(&self, key: impl Into<String>, value: impl Into<String>) -> Result<()> {
        let updated = self.stack.with_frame_mut(self.id, |frame| {
            frame.variables.insert(key.into(), value.into());
        });
        if updated.is_some() {
            Ok(())
        } else {
            Err(AgentFlowError::Context(format!(
                "scope {:?} is no longer active",
                self.kind.as_str()
            )))
        }
    }

    pub async fn get(&self, key: &str) -> Option<String> {
        self.stack
            .with_frame_mut(self.id, |frame| frame.variables.get(key).cloned())
            .unwrap_or(None)
    }

    pub async fn remove(&self, key: &str) -> Result<()> {
        let updated = self.stack.with_frame_mut(self.id, |frame| {
            frame.variables.remove(key);
        });
        if updated.is_some() {
            Ok(())
        } else {
            Err(AgentFlowError::Context(format!(
                "scope {:?} is no longer active",
                self.kind.as_str()
            )))
        }
    }
}

impl Drop for FlowScopeGuard {
    fn drop(&mut self) {
        self.stack.remove(self.id);
    }
}

#[derive(Clone)]
pub struct FlowVariables {
    stack: Arc<ScopeStack>,
    global_scope_id: ScopeId,
}

impl FlowVariables {
    pub async fn set_global(&self, key: impl Into<String>, value: impl Into<String>) -> Result<()> {
        let updated = self.stack.with_frame_mut(self.global_scope_id, |frame| {
            frame.variables.insert(key.into(), value.into());
        });
        if updated.is_some() {
            Ok(())
        } else {
            Err(AgentFlowError::Context(
                "global scope is not available".to_string(),
            ))
        }
    }

    pub async fn get_global(&self, key: &str) -> Option<String> {
        self.stack
            .with_frame_mut(self.global_scope_id, |frame| {
                frame.variables.get(key).cloned()
            })
            .unwrap_or(None)
    }

    pub async fn remove_global(&self, key: &str) -> Result<()> {
        let updated = self.stack.with_frame_mut(self.global_scope_id, |frame| {
            frame.variables.remove(key);
        });
        if updated.is_some() {
            Ok(())
        } else {
            Err(AgentFlowError::Context(
                "global scope is not available".to_string(),
            ))
        }
    }

    pub async fn get(&self, key: &str) -> Option<String> {
        self.stack.with_frames(|frames| {
            frames
                .iter()
                .rev()
                .find_map(|frame| frame.variables.get(key).cloned())
        })
    }
}
