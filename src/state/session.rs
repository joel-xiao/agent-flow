use super::store::ContextStore;
use crate::error::Result;
use std::sync::Arc;

const SESSION_PREFIX: &str = "session";

/// 会话上下文
#[derive(Clone)]
pub struct SessionContext {
    pub store: Arc<dyn ContextStore>,
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
