use serde_json::json;
use crate::error::{Result, AgentFlowError};
use anyhow::anyhow;
use super::core::GenericApiClient;
use super::super::types::*;

/// Chat 相关 API 实现

pub async fn chat_completion(
    client: &GenericApiClient,
    request: ChatCompletionRequest,
) -> Result<ChatCompletionResponse> {
    let body = json!({
        "model": request.model,
        "messages": request.messages,
        "temperature": request.temperature,
        "max_tokens": request.max_tokens,
        "stream": request.stream.unwrap_or(false),
    });

    let response = client.request("POST", "chat_completion", Some(body)).await
        .map_err(|e| AgentFlowError::Other(anyhow!("Request failed: {}", e)))?;
    
    let content = response["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!(
            "Missing content in response: {}",
            serde_json::to_string(&response).unwrap_or_default()
        )))?;

    Ok(ChatCompletionResponse {
        content: content.to_string(),
        metadata: response,
    })
}

pub async fn chat_completion_async(
    client: &GenericApiClient,
    request: ChatCompletionRequest,
) -> Result<AsyncTaskResponse> {
    let body = json!({
        "model": request.model,
        "messages": request.messages,
        "temperature": request.temperature,
        "max_tokens": request.max_tokens,
    });

    let response = client.request("POST", "chat_completion_async", Some(body)).await
        .map_err(|e| AgentFlowError::Other(anyhow!("Request failed: {}", e)))?;
    
    Ok(AsyncTaskResponse {
        task_id: response["task_id"].as_str()
            .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Missing task_id")))?
            .to_string(),
        status: response["status"].as_str()
            .unwrap_or("pending")
            .to_string(),
        metadata: response,
    })
}

pub async fn get_async_result(
    client: &GenericApiClient,
    task_id: &str,
) -> Result<AsyncTaskResult> {
    let url = client.config.get_endpoint("get_async_result")
        .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Endpoint not configured")))?
        .replace("{task_id}", task_id);

    let response = client.client
        .get(&url)
        .header("Authorization", format!("Bearer {}", client.api_key))
        .send()
        .await
        .map_err(|e| AgentFlowError::Other(anyhow!("HTTP request error: {}", e)))?;

    let payload: serde_json::Value = response.json().await
        .map_err(|e| AgentFlowError::Other(anyhow!("Response parse error: {}", e)))?;
    
    Ok(AsyncTaskResult {
        task_id: task_id.to_string(),
        status: payload["status"].as_str().unwrap_or("unknown").to_string(),
        result: payload.get("result").cloned(),
        error: payload.get("error").and_then(|v| v.as_str()).map(|s| s.to_string()),
    })
}

