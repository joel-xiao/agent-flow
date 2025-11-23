use std::sync::Arc;

use async_trait::async_trait;

use crate::error::Result;
use super::client::{LlmClient, DynLlmClient};
use super::types::{LlmRequest, LlmResponse};

#[derive(Default, Clone)]
pub struct LocalEchoClient;

#[async_trait]
impl LlmClient for LocalEchoClient {
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse> {
        Ok(LlmResponse {
            content: format!("[Echo] {}", request.user),
            metadata: None,
        })
    }

    fn clone_dyn(&self) -> DynLlmClient {
        Arc::new(LocalEchoClient)
    }
}

