use std::sync::Arc;
use parking_lot::RwLock;
use crate::agent::AgentMessage;
use crate::error::Result;
use super::store::ContextStore;
use super::scope::{ScopeStack, FlowScopeKind, ScopeId};

/// Flow 上下文
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

    pub fn session(&self) -> super::session::SessionContext {
        super::session::SessionContext {
            store: Arc::clone(&self.store),
        }
    }

    pub fn scope(&self, kind: FlowScopeKind) -> super::scope::FlowScopeGuard {
        let scope_id = self.scopes.push_scope(kind.clone());
        super::scope::FlowScopeGuard::new(
            Arc::clone(&self.scopes),
            scope_id,
            kind,
        )
    }

    pub fn variables(&self) -> super::scope::FlowVariables {
        super::scope::FlowVariables::new(
            Arc::clone(&self.scopes),
            self.global_scope_id,
        )
    }
}

