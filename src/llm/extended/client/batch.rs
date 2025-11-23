use serde_json::json;
use crate::error::{Result, AgentFlowError};
use anyhow::anyhow;
use super::core::GenericApiClient;
use super::super::types::*;

/// 批处理相关 API 实现

pub async fn list_batch_tasks(
    client: &GenericApiClient,
) -> Result<BatchTaskListResponse> {
    let response = client.request("GET", "list_batch_tasks", None).await
        .map_err(|e| AgentFlowError::Other(anyhow!("Request failed: {}", e)))?;
    
    let tasks = response["tasks"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|v| {
            Some(BatchTaskInfo {
                id: v["id"].as_str()?.to_string(),
                status: v["status"].as_str()?.to_string(),
                created_at: v["created_at"].as_str()?.to_string(),
                metadata: v.clone(),
            })
        })
        .collect();

    Ok(BatchTaskListResponse {
        tasks,
        metadata: response,
    })
}

pub async fn create_batch_task(
    client: &GenericApiClient,
    request: BatchTaskRequest,
) -> Result<BatchTaskResponse> {
    let body = json!({
        "tasks": request.tasks,
    });

    let response = client.request("POST", "create_batch_task", Some(body)).await
        .map_err(|e| AgentFlowError::Other(anyhow!("Request failed: {}", e)))?;
    
    Ok(BatchTaskResponse {
        task_id: response["task_id"].as_str()
            .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Missing task_id")))?
            .to_string(),
        status: response["status"].as_str().unwrap_or("pending").to_string(),
        metadata: response,
    })
}

pub async fn get_batch_task(
    client: &GenericApiClient,
    task_id: &str,
) -> Result<BatchTaskResponse> {
    let url = client.config.get_endpoint("get_batch_task")
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
    
    Ok(BatchTaskResponse {
        task_id: task_id.to_string(),
        status: payload["status"].as_str().unwrap_or("unknown").to_string(),
        metadata: payload,
    })
}

pub async fn cancel_batch_task(
    client: &GenericApiClient,
    task_id: &str,
) -> Result<()> {
    let url = client.config.get_endpoint("cancel_batch_task")
        .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Endpoint not configured")))?
        .replace("{task_id}", task_id);

    client.client
        .post(&url)
        .header("Authorization", format!("Bearer {}", client.api_key))
        .send()
        .await
        .map_err(|e| AgentFlowError::Other(anyhow!("HTTP request error: {}", e)))?;

    Ok(())
}

