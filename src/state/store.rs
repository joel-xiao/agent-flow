use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use parking_lot::RwLock;
use crate::error::Result;

/// 上下文存储 trait
#[async_trait]
pub trait ContextStore: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<String>>;
    async fn set(&self, key: &str, value: String) -> Result<()>;
    async fn delete(&self, key: &str) -> Result<()>;
}

/// 内存存储实现
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

