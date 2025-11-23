use serde_json::json;
use crate::error::{Result, AgentFlowError};
use anyhow::anyhow;
use super::core::GenericApiClient;
use super::super::types::*;

/// 文件相关 API 实现

pub async fn parse_file_sync(
    client: &GenericApiClient,
    request: FileParseRequest,
) -> Result<FileParseResponse> {
    let body = json!({
        "file_id": request.file_id,
        "parse_type": request.parse_type,
    });

    let response = client.request("POST", "parse_file_sync", Some(body)).await
        .map_err(|e| AgentFlowError::Other(anyhow!("Request failed: {}", e)))?;
    
    Ok(FileParseResponse {
        content: response["content"].as_str()
            .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Missing content")))?
            .to_string(),
        metadata: response,
    })
}

pub async fn parse_file_async(
    client: &GenericApiClient,
    request: FileParseRequest,
) -> Result<AsyncTaskResponse> {
    let body = json!({
        "file_id": request.file_id,
        "parse_type": request.parse_type,
    });

    let response = client.request("POST", "parse_file_async", Some(body)).await
        .map_err(|e| AgentFlowError::Other(anyhow!("Request failed: {}", e)))?;
    
    Ok(AsyncTaskResponse {
        task_id: response["task_id"].as_str()
            .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Missing task_id")))?
            .to_string(),
        status: response["status"].as_str().unwrap_or("pending").to_string(),
        metadata: response,
    })
}

pub async fn list_files(
    client: &GenericApiClient,
) -> Result<FileListResponse> {
    let response = client.request("GET", "list_files", None).await
        .map_err(|e| AgentFlowError::Other(anyhow!("Request failed: {}", e)))?;
    
    let files = response["files"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|v| {
            Some(FileInfo {
                id: v["id"].as_str()?.to_string(),
                name: v["name"].as_str()?.to_string(),
                size: v["size"].as_u64()?,
                mime_type: v["mime_type"].as_str()?.to_string(),
                created_at: v["created_at"].as_str()?.to_string(),
                metadata: v.clone(),
            })
        })
        .collect();

    Ok(FileListResponse {
        files,
        metadata: response,
    })
}

pub async fn upload_file(
    client: &GenericApiClient,
    request: FileUploadRequest,
) -> Result<FileUploadResponse> {
    let form = reqwest::multipart::Form::new()
        .part("file", reqwest::multipart::Part::bytes(request.content)
            .file_name(request.name.clone())
            .mime_str(request.mime_type.as_deref().unwrap_or("application/octet-stream"))
            .map_err(|e| AgentFlowError::Other(anyhow!("Invalid mime type: {}", e)))?)
        .text("name", request.name);

    let url = client.config.get_endpoint("upload_file")
        .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Endpoint not configured")))?;

    let response = client.client
        .post(&url)
        .header("Authorization", format!("Bearer {}", client.api_key))
        .multipart(form)
        .send()
        .await
        .map_err(|e| AgentFlowError::Other(anyhow!("HTTP request error: {}", e)))?;

    let payload: serde_json::Value = response.json().await
        .map_err(|e| AgentFlowError::Other(anyhow!("Response parse error: {}", e)))?;
    
    Ok(FileUploadResponse {
        file_id: payload["file_id"].as_str()
            .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Missing file_id")))?
            .to_string(),
        metadata: payload,
    })
}

pub async fn delete_file(
    client: &GenericApiClient,
    file_id: &str,
) -> Result<()> {
    let url = client.config.get_endpoint("delete_file")
        .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Endpoint not configured")))?
        .replace("{file_id}", file_id);

    client.client
        .delete(&url)
        .header("Authorization", format!("Bearer {}", client.api_key))
        .send()
        .await
        .map_err(|e| AgentFlowError::Other(anyhow!("HTTP request error: {}", e)))?;

    Ok(())
}

pub async fn get_file_content(
    client: &GenericApiClient,
    file_id: &str,
) -> Result<FileContentResponse> {
    let url = client.config.get_endpoint("get_file_content")
        .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Endpoint not configured")))?
        .replace("{file_id}", file_id);

    let response = client.client
        .get(&url)
        .header("Authorization", format!("Bearer {}", client.api_key))
        .send()
        .await
        .map_err(|e| AgentFlowError::Other(anyhow!("HTTP request error: {}", e)))?;

    let mime_type = response.headers()
        .get("content-type")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();
    let content = response.bytes().await
        .map_err(|e| AgentFlowError::Other(anyhow!("Failed to read response: {}", e)))?
        .to_vec();

    Ok(FileContentResponse {
        content,
        mime_type,
        metadata: json!({}),
    })
}

