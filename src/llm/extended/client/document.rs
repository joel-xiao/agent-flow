use serde_json::json;
use crate::error::{Result, AgentFlowError};
use anyhow::anyhow;
use super::core::GenericApiClient;
use super::super::types::*;

/// 文档相关 API 实现

pub async fn list_documents(
    client: &GenericApiClient,
    kb_id: &str,
) -> Result<DocumentListResponse> {
    let url = client.config.get_endpoint("list_documents")
        .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Endpoint not configured")))?
        .replace("{kb_id}", kb_id);

    let response = client.client
        .get(&url)
        .header("Authorization", format!("Bearer {}", client.api_key))
        .send()
        .await
        .map_err(|e| AgentFlowError::Other(anyhow!("HTTP request error: {}", e)))?;

    let payload: serde_json::Value = response.json().await
        .map_err(|e| AgentFlowError::Other(anyhow!("Response parse error: {}", e)))?;
    
    let documents = payload["documents"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|v| {
            Some(DocumentInfo {
                id: v["id"].as_str()?.to_string(),
                name: v["name"].as_str()?.to_string(),
                status: v["status"].as_str()?.to_string(),
                created_at: v["created_at"].as_str()?.to_string(),
                metadata: v.clone(),
            })
        })
        .collect();

    Ok(DocumentListResponse {
        documents,
        metadata: payload,
    })
}

pub async fn upload_document_file(
    client: &GenericApiClient,
    kb_id: &str,
    request: DocumentUploadRequest,
) -> Result<DocumentResponse> {
    let form = reqwest::multipart::Form::new()
        .part("file", reqwest::multipart::Part::bytes(request.content)
            .file_name(request.name.clone())
            .mime_str(request.mime_type.as_deref().unwrap_or("application/octet-stream"))
            .map_err(|e| AgentFlowError::Other(anyhow!("Invalid mime type: {}", e)))?)
        .text("name", request.name);

    let url = client.config.get_endpoint("upload_document_file")
        .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Endpoint not configured")))?
        .replace("{kb_id}", kb_id);

    let response = client.client
        .post(&url)
        .header("Authorization", format!("Bearer {}", client.api_key))
        .multipart(form)
        .send()
        .await
        .map_err(|e| AgentFlowError::Other(anyhow!("HTTP request error: {}", e)))?;

    let payload: serde_json::Value = response.json().await
        .map_err(|e| AgentFlowError::Other(anyhow!("Response parse error: {}", e)))?;
    
    Ok(DocumentResponse {
        id: payload["id"].as_str()
            .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Missing id")))?
            .to_string(),
        name: payload["name"].as_str()
            .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Missing name")))?
            .to_string(),
        status: payload["status"].as_str().unwrap_or("pending").to_string(),
        metadata: payload,
    })
}

pub async fn upload_document_url(
    client: &GenericApiClient,
    kb_id: &str,
    request: DocumentUrlRequest,
) -> Result<DocumentResponse> {
    let body = json!({
        "url": request.url,
        "name": request.name,
    });

    let url = client.config.get_endpoint("upload_document_url")
        .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Endpoint not configured")))?
        .replace("{kb_id}", kb_id);

    let response = client.client
        .post(&url)
        .header("Authorization", format!("Bearer {}", client.api_key))
        .json(&body)
        .send()
        .await
        .map_err(|e| AgentFlowError::Other(anyhow!("HTTP request error: {}", e)))?;

    let payload: serde_json::Value = response.json().await
        .map_err(|e| AgentFlowError::Other(anyhow!("Response parse error: {}", e)))?;
    
    Ok(DocumentResponse {
        id: payload["id"].as_str()
            .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Missing id")))?
            .to_string(),
        name: payload["name"].as_str()
            .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Missing name")))?
            .to_string(),
        status: payload["status"].as_str().unwrap_or("pending").to_string(),
        metadata: payload,
    })
}

pub async fn parse_document_image(
    client: &GenericApiClient,
    kb_id: &str,
    request: DocumentImageParseRequest,
) -> Result<DocumentResponse> {
    let body = json!({
        "image_url": request.image_url,
        "name": request.name,
    });

    let url = client.config.get_endpoint("parse_document_image")
        .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Endpoint not configured")))?
        .replace("{kb_id}", kb_id);

    let response = client.client
        .post(&url)
        .header("Authorization", format!("Bearer {}", client.api_key))
        .json(&body)
        .send()
        .await
        .map_err(|e| AgentFlowError::Other(anyhow!("HTTP request error: {}", e)))?;

    let payload: serde_json::Value = response.json().await
        .map_err(|e| AgentFlowError::Other(anyhow!("Response parse error: {}", e)))?;
    
    Ok(DocumentResponse {
        id: payload["id"].as_str()
            .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Missing id")))?
            .to_string(),
        name: payload["name"].as_str()
            .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Missing name")))?
            .to_string(),
        status: payload["status"].as_str().unwrap_or("pending").to_string(),
        metadata: payload,
    })
}

pub async fn get_document(
    client: &GenericApiClient,
    kb_id: &str,
    doc_id: &str,
) -> Result<DocumentResponse> {
    let url = client.config.get_endpoint("get_document")
        .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Endpoint not configured")))?
        .replace("{kb_id}", kb_id)
        .replace("{doc_id}", doc_id);

    let response = client.client
        .get(&url)
        .header("Authorization", format!("Bearer {}", client.api_key))
        .send()
        .await
        .map_err(|e| AgentFlowError::Other(anyhow!("HTTP request error: {}", e)))?;

    let payload: serde_json::Value = response.json().await
        .map_err(|e| AgentFlowError::Other(anyhow!("Response parse error: {}", e)))?;
    
    Ok(DocumentResponse {
        id: doc_id.to_string(),
        name: payload["name"].as_str()
            .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Missing name")))?
            .to_string(),
        status: payload["status"].as_str().unwrap_or("unknown").to_string(),
        metadata: payload,
    })
}

pub async fn delete_document(
    client: &GenericApiClient,
    kb_id: &str,
    doc_id: &str,
) -> Result<()> {
    let url = client.config.get_endpoint("delete_document")
        .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Endpoint not configured")))?
        .replace("{kb_id}", kb_id)
        .replace("{doc_id}", doc_id);

    client.client
        .delete(&url)
        .header("Authorization", format!("Bearer {}", client.api_key))
        .send()
        .await
        .map_err(|e| AgentFlowError::Other(anyhow!("HTTP request error: {}", e)))?;

    Ok(())
}

pub async fn reindex_document(
    client: &GenericApiClient,
    kb_id: &str,
    doc_id: &str,
) -> Result<()> {
    let url = client.config.get_endpoint("reindex_document")
        .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Endpoint not configured")))?
        .replace("{kb_id}", kb_id)
        .replace("{doc_id}", doc_id);

    client.client
        .post(&url)
        .header("Authorization", format!("Bearer {}", client.api_key))
        .send()
        .await
        .map_err(|e| AgentFlowError::Other(anyhow!("HTTP request error: {}", e)))?;

    Ok(())
}

