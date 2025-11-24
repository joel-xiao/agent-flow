use crate::error::{AgentFlowError, Result};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Scope ID 类型
pub type ScopeId = u64;

/// Flow Scope 类型
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FlowScopeKind {
    Global,
    Node(String),
    Branch(String),
    Custom(String),
}

impl FlowScopeKind {
    pub fn as_str(&self) -> &str {
        match self {
            FlowScopeKind::Global => "global",
            FlowScopeKind::Node(_) => "node",
            FlowScopeKind::Branch(_) => "branch",
            FlowScopeKind::Custom(_) => "custom",
        }
    }
}

/// Scope 栈
#[derive(Default)]
pub struct ScopeStack {
    frames: RwLock<Vec<ScopeFrame>>,
}

impl ScopeStack {
    pub fn push_scope(&self, kind: FlowScopeKind) -> ScopeId {
        let id = NEXT_SCOPE_ID.fetch_add(1, Ordering::Relaxed);
        self.frames.write().push(ScopeFrame::new(id, kind));
        id
    }

    pub fn remove(&self, id: ScopeId) {
        let mut frames = self.frames.write();
        if let Some(pos) = frames.iter().rposition(|frame| frame.id == id) {
            frames.remove(pos);
        }
    }

    pub fn with_frame_mut<R>(
        &self,
        id: ScopeId,
        apply: impl FnOnce(&mut ScopeFrame) -> R,
    ) -> Option<R> {
        let mut frames = self.frames.write();
        frames.iter_mut().find(|frame| frame.id == id).map(apply)
    }

    pub fn with_frames<R>(&self, apply: impl FnOnce(&[ScopeFrame]) -> R) -> R {
        let frames = self.frames.read();
        apply(&frames)
    }
}

/// Scope 帧
#[derive(Clone)]
pub struct ScopeFrame {
    pub id: ScopeId,
    pub kind: FlowScopeKind,
    pub variables: HashMap<String, String>,
}

impl ScopeFrame {
    pub fn new(id: ScopeId, kind: FlowScopeKind) -> Self {
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

/// Flow Scope Guard
#[derive(Clone)]
pub struct FlowScopeGuard {
    stack: Arc<ScopeStack>,
    id: ScopeId,
    kind: FlowScopeKind,
}

impl FlowScopeGuard {
    pub fn new(stack: Arc<ScopeStack>, id: ScopeId, kind: FlowScopeKind) -> Self {
        Self { stack, id, kind }
    }

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

/// Flow 变量管理器
#[derive(Clone)]
pub struct FlowVariables {
    stack: Arc<ScopeStack>,
    global_scope_id: ScopeId,
}

impl FlowVariables {
    pub fn new(stack: Arc<ScopeStack>, global_scope_id: ScopeId) -> Self {
        Self {
            stack,
            global_scope_id,
        }
    }

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

    pub async fn set(&self, key: impl Into<String>, value: impl Into<String>) -> Result<()> {
        let frame_id = {
            let frames = self.stack.frames.read();
            if let Some(frame) = frames.last() {
                frame.id.clone()
            } else {
                return Err(AgentFlowError::Context("no active scope".to_string()));
            }
        };
        let updated = self.stack.with_frame_mut(frame_id, |frame| {
            frame.variables.insert(key.into(), value.into());
        });
        if updated.is_some() {
            Ok(())
        } else {
            Err(AgentFlowError::Context("no active scope".to_string()))
        }
    }
}
