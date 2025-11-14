use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::anyhow;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::time::timeout;

use crate::error::{AgentFlowError, Result};
use crate::llm::DynLlmClient;

#[derive(Clone)]
pub struct ModelHandle {
    name: String,
    client: DynLlmClient,
    timeout: Option<Duration>,
}

impl ModelHandle {
    pub async fn run<F, Fut, T>(&self, task: F) -> Result<T>
    where
        F: FnOnce(&DynLlmClient) -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let fut = task(&self.client);
        if let Some(duration) = self.timeout {
            timeout(duration, fut)
                .await
                .map_err(|_| AgentFlowError::Other(anyhow!("model `{}` timed out", self.name)))?
        } else {
            fut.await
        }
    }
}

#[derive(Clone)]
pub struct ModelSpec {
    pub name: String,
    pub max_concurrency: usize,
    pub timeout: Option<Duration>,
}

#[derive(Default)]
pub struct ModelRegistry {
    specs: HashMap<String, ModelSpec>,
    clients: HashMap<String, DynLlmClient>,
    semaphores: HashMap<String, Arc<Semaphore>>,
}

impl ModelRegistry {
    pub fn new() -> Self {
        Self {
            specs: HashMap::new(),
            clients: HashMap::new(),
            semaphores: HashMap::new(),
        }
    }

    pub fn register_model(&mut self, spec: ModelSpec, client: DynLlmClient) {
        let name = spec.name.clone();
        self.semaphores.insert(
            name.clone(),
            Arc::new(Semaphore::new(spec.max_concurrency.max(1))),
        );
        self.clients.insert(name.clone(), client);
        self.specs.insert(name, spec);
    }

    pub async fn checkout(&self, name: &str) -> Result<ModelGuard> {
        let semaphore = self
            .semaphores
            .get(name)
            .ok_or_else(|| AgentFlowError::Other(anyhow!("model `{}` not registered", name)))?;
        let permit = semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| AgentFlowError::Other(anyhow!("model `{}` closed", name)))?;
        let client = self
            .clients
            .get(name)
            .ok_or_else(|| AgentFlowError::Other(anyhow!("model `{}` client missing", name)))?;
        let spec = self
            .specs
            .get(name)
            .ok_or_else(|| AgentFlowError::Other(anyhow!("model `{}` spec missing", name)))?;

        Ok(ModelGuard {
            handle: ModelHandle {
                name: spec.name.clone(),
                client: Arc::clone(client),
                timeout: spec.timeout,
            },
            permit: Some(permit),
        })
    }
}

pub struct ModelGuard {
    handle: ModelHandle,
    permit: Option<OwnedSemaphorePermit>,
}

impl ModelGuard {
    pub fn handle(&self) -> &ModelHandle {
        &self.handle
    }

    pub async fn run<F, Fut, T>(&self, task: F) -> Result<T>
    where
        F: FnOnce(&DynLlmClient) -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        self.handle.run(task).await
    }
}

impl Drop for ModelGuard {
    fn drop(&mut self) {
        if let Some(permit) = self.permit.take() {
            drop(permit);
        }
    }
}

#[derive(Default)]
pub struct ToolResourceManager {
    pools: HashMap<String, ResourcePool>,
}

impl ToolResourceManager {
    pub fn new() -> Self {
        Self {
            pools: HashMap::new(),
        }
    }

    pub fn register_semaphore_pool(&mut self, name: impl Into<String>, limit: usize) {
        self.pools.insert(
            name.into(),
            ResourcePool::Semaphore {
                semaphore: Arc::new(Semaphore::new(limit.max(1))),
            },
        );
    }

    pub async fn acquire(&self, name: &str) -> Result<ResourceHandle> {
        match self.pools.get(name) {
            Some(ResourcePool::Semaphore { semaphore }) => {
                let permit =
                    semaphore.clone().acquire_owned().await.map_err(|_| {
                        AgentFlowError::Other(anyhow!("resource `{}` closed", name))
                    })?;
                Ok(ResourceHandle::Semaphore {
                    permit: Some(permit),
                })
            }
            None => Err(AgentFlowError::Other(anyhow!(
                "resource `{name}` not registered"
            ))),
        }
    }
}

pub enum ResourcePool {
    Semaphore { semaphore: Arc<Semaphore> },
}

pub enum ResourceHandle {
    Semaphore {
        permit: Option<OwnedSemaphorePermit>,
    },
}

impl ResourceHandle {
    pub async fn run<F, Fut, T>(&self, task: F) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        task().await
    }
}

impl Drop for ResourceHandle {
    fn drop(&mut self) {
        match self {
            ResourceHandle::Semaphore { permit } => {
                if let Some(permit) = permit.take() {
                    drop(permit);
                }
            }
        }
    }
}
