use serde_json::json;
use crate::error::{Result, AgentFlowError};
use anyhow::anyhow;
use super::core::GenericApiClient;
use super::super::types::*;

/// Assistant 相关 API 实现

pub async fn assistant_chat(
    client: &GenericApiClient,
    request: AssistantChatRequest,
) -> Result<AssistantChatResponse> {
    let body = json!({
        "assistant_id": request.assistant_id,
        "message": request.message,
        "session_id": request.session_id,
    });

    let response = client.request("POST", "assistant_chat", Some(body)).await
        .map_err(|e| AgentFlowError::Other(anyhow!("Request failed: {}", e)))?;
    
    Ok(AssistantChatResponse {
        response: response["response"].as_str()
            .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Missing response")))?
            .to_string(),
        session_id: response["session_id"].as_str()
            .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Missing session_id")))?
            .to_string(),
        metadata: response,
    })
}

