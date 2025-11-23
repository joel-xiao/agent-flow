use serde_json::json;
use crate::error::{Result, AgentFlowError};
use anyhow::anyhow;
use super::core::GenericApiClient;
use super::super::types::*;

/// Agent 相关 API 实现

pub async fn agent_chat(
    client: &GenericApiClient,
    request: AgentChatRequest,
) -> Result<AgentChatResponse> {
    let body = json!({
        "agent_id": request.agent_id,
        "message": request.message,
        "session_id": request.session_id,
    });

    let response = client.request("POST", "agent_chat", Some(body)).await
        .map_err(|e| AgentFlowError::Other(anyhow!("Request failed: {}", e)))?;
    
    Ok(AgentChatResponse {
        response: response["response"].as_str()
            .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Missing response")))?
            .to_string(),
        session_id: response["session_id"].as_str()
            .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Missing session_id")))?
            .to_string(),
        metadata: response,
    })
}

pub async fn agent_chat_async(
    client: &GenericApiClient,
    request: AgentChatRequest,
) -> Result<AsyncTaskResponse> {
    let body = json!({
        "agent_id": request.agent_id,
        "message": request.message,
        "session_id": request.session_id,
    });

    let response = client.request("POST", "agent_chat_async", Some(body)).await
        .map_err(|e| AgentFlowError::Other(anyhow!("Request failed: {}", e)))?;
    
    Ok(AsyncTaskResponse {
        task_id: response["task_id"].as_str()
            .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Missing task_id")))?
            .to_string(),
        status: response["status"].as_str().unwrap_or("pending").to_string(),
        metadata: response,
    })
}

pub async fn agent_history(
    client: &GenericApiClient,
    session_id: &str,
) -> Result<AgentHistoryResponse> {
    let url = client.config.get_endpoint("agent_history")
        .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Endpoint not configured")))?
        .replace("{session_id}", session_id);

    let response = client.client
        .get(&url)
        .header("Authorization", format!("Bearer {}", client.api_key))
        .send()
        .await
        .map_err(|e| AgentFlowError::Other(anyhow!("HTTP request error: {}", e)))?;

    let payload: serde_json::Value = response.json().await
        .map_err(|e| AgentFlowError::Other(anyhow!("Response parse error: {}", e)))?;
    
    Ok(AgentHistoryResponse {
        messages: payload["messages"].as_array()
            .unwrap_or(&vec![])
            .to_vec(),
        metadata: payload,
    })
}

