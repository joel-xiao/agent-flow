use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures::{Stream, StreamExt};

use super::types::{LlmRequest, LlmResponse, LlmStreamChunk};
use crate::error::{AgentFlowError, Result};

pub type LlmStream = Pin<Box<dyn Stream<Item = Result<LlmStreamChunk>> + Send>>;

#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse>;

    fn complete_stream(&self, request: LlmRequest) -> LlmStream {
        let request = Arc::new(request);
        let client = self.clone_dyn();

        Box::pin(
            futures::stream::unfold(
                (request, client, None::<String>, 0usize),
                move |(req, client, mut full_content, mut pos)| async move {
                    if full_content.is_none() {
                        match client.complete((*req).clone()).await {
                            Ok(response) => {
                                full_content = Some(response.content);
                                pos = 0;
                            }
                            Err(e) => {
                                return Some((Err(e), (req, client, full_content, pos)));
                            }
                        }
                    }

                    let content = match full_content.as_ref() {
                        Some(c) => c,
                        None => {
                            return Some((
                                Err(AgentFlowError::Other(anyhow::anyhow!(
                                    "Stream response content is empty"
                                ))
                                .into()),
                                (req, client, full_content, pos),
                            ))
                        }
                    };
                    if pos < content.len() {
                        let char_start = pos;
                        let ch = match content[char_start..].chars().next() {
                            Some(c) => c,
                            None => {
                                return Some((
                                    Err(AgentFlowError::Other(anyhow::anyhow!(
                                        "Character parsing failed"
                                    ))
                                    .into()),
                                    (req, client, full_content, pos),
                                ))
                            }
                        };
                        pos += ch.len_utf8();

                        Some((
                            Ok(LlmStreamChunk {
                                content: ch.to_string(),
                                done: false,
                            }),
                            (req, client, full_content, pos),
                        ))
                    } else {
                        None
                    }
                },
            )
            .chain(futures::stream::once(async move {
                Ok(LlmStreamChunk {
                    content: String::new(),
                    done: true,
                })
            })),
        )
    }

    fn clone_dyn(&self) -> Arc<dyn LlmClient>;
}

pub type DynLlmClient = Arc<dyn LlmClient>;
