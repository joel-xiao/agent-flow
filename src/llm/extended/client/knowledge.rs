use serde_json::json;
use crate::error::{Result, AgentFlowError};
use anyhow::anyhow;
use super::core::GenericApiClient;
use super::super::types::*;

/// 知识库相关 API 实现

pub async fn knowledge_retrieve(
    client: &GenericApiClient,
    request: KnowledgeRetrieveRequest,
) -> Result<KnowledgeRetrieveResponse> {
    let body = json!({
        "kb_id": request.kb_id,
        "query": request.query,
        "top_k": request.top_k,
    });

    let url = client.config.get_endpoint("knowledge_retrieve")
        .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Endpoint not configured")))?;

    let response = client.client
        .post(&url)
        .header("Authorization", format!("Bearer {}", client.api_key))
        .json(&body)
        .send()
        .await
        .map_err(|e| AgentFlowError::Other(anyhow!("HTTP request error: {}", e)))?;

    let payload: serde_json::Value = response.json().await
        .map_err(|e| AgentFlowError::Other(anyhow!("Response parse error: {}", e)))?;
    
    let results = payload["results"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|v| {
            Some(KnowledgeResult {
                content: v["content"].as_str()?.to_string(),
                score: v["score"].as_f64()? as f32,
                source: v["source"].as_str().map(|s| s.to_string()),
                metadata: v.clone(),
            })
        })
        .collect();

    Ok(KnowledgeRetrieveResponse {
        results,
        metadata: payload,
    })
}

pub async fn list_knowledge_bases(
    client: &GenericApiClient,
) -> Result<KnowledgeBaseListResponse> {
    let response = client.request("GET", "list_knowledge_bases", None).await
        .map_err(|e| AgentFlowError::Other(anyhow!("Request failed: {}", e)))?;
    
    let knowledge_bases = response["knowledge_bases"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|v| {
            Some(KnowledgeBaseInfo {
                id: v["id"].as_str()?.to_string(),
                name: v["name"].as_str()?.to_string(),
                created_at: v["created_at"].as_str()?.to_string(),
                metadata: v.clone(),
            })
        })
        .collect();

    Ok(KnowledgeBaseListResponse {
        knowledge_bases,
        metadata: response,
    })
}

pub async fn create_knowledge_base(
    client: &GenericApiClient,
    request: KnowledgeBaseCreateRequest,
) -> Result<KnowledgeBaseResponse> {
    let body = json!({
        "name": request.name,
        "description": request.description,
    });

    let response = client.request("POST", "create_knowledge_base", Some(body)).await
        .map_err(|e| AgentFlowError::Other(anyhow!("Request failed: {}", e)))?;
    
    Ok(KnowledgeBaseResponse {
        id: response["id"].as_str()
            .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Missing id")))?
            .to_string(),
        name: response["name"].as_str()
            .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Missing name")))?
            .to_string(),
        description: response["description"].as_str().map(|s| s.to_string()),
        created_at: response["created_at"].as_str()
            .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Missing created_at")))?
            .to_string(),
        metadata: response,
    })
}

pub async fn get_knowledge_base(
    client: &GenericApiClient,
    kb_id: &str,
) -> Result<KnowledgeBaseResponse> {
    let url = client.config.get_endpoint("get_knowledge_base")
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
    
    Ok(KnowledgeBaseResponse {
        id: kb_id.to_string(),
        name: payload["name"].as_str()
            .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Missing name")))?
            .to_string(),
        description: payload["description"].as_str().map(|s| s.to_string()),
        created_at: payload["created_at"].as_str()
            .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Missing created_at")))?
            .to_string(),
        metadata: payload,
    })
}

pub async fn update_knowledge_base(
    client: &GenericApiClient,
    kb_id: &str,
    request: KnowledgeBaseUpdateRequest,
) -> Result<KnowledgeBaseResponse> {
    let body = json!({
        "name": request.name,
        "description": request.description,
    });

    let url = client.config.get_endpoint("update_knowledge_base")
        .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Endpoint not configured")))?
        .replace("{kb_id}", kb_id);

    let response = client.client
        .put(&url)
        .header("Authorization", format!("Bearer {}", client.api_key))
        .json(&body)
        .send()
        .await
        .map_err(|e| AgentFlowError::Other(anyhow!("HTTP request error: {}", e)))?;

    let payload: serde_json::Value = response.json().await
        .map_err(|e| AgentFlowError::Other(anyhow!("Response parse error: {}", e)))?;
    
    Ok(KnowledgeBaseResponse {
        id: kb_id.to_string(),
        name: payload["name"].as_str()
            .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Missing name")))?
            .to_string(),
        description: payload["description"].as_str().map(|s| s.to_string()),
        created_at: payload["created_at"].as_str()
            .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Missing created_at")))?
            .to_string(),
        metadata: payload,
    })
}

pub async fn delete_knowledge_base(
    client: &GenericApiClient,
    kb_id: &str,
) -> Result<()> {
    let url = client.config.get_endpoint("delete_knowledge_base")
        .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Endpoint not configured")))?
        .replace("{kb_id}", kb_id);

    client.client
        .delete(&url)
        .header("Authorization", format!("Bearer {}", client.api_key))
        .send()
        .await
        .map_err(|e| AgentFlowError::Other(anyhow!("HTTP request error: {}", e)))?;

    Ok(())
}

pub async fn get_knowledge_base_usage(
    client: &GenericApiClient,
    kb_id: &str,
) -> Result<KnowledgeBaseUsageResponse> {
    let url = client.config.get_endpoint("get_knowledge_base_usage")
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
    
    Ok(KnowledgeBaseUsageResponse {
        usage: payload["usage"].clone(),
        metadata: payload,
    })
}

