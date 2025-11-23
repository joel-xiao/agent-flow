use serde_json::json;
use crate::error::{Result, AgentFlowError};
use anyhow::anyhow;
use super::core::GenericApiClient;
use super::super::types::*;

/// 内容安全相关 API 实现

pub async fn content_safety(
    client: &GenericApiClient,
    request: ContentSafetyRequest,
) -> Result<ContentSafetyResponse> {
    let body = json!({
        "text": request.text,
        "categories": request.categories,
    });

    let response = client.request("POST", "content_safety", Some(body)).await
        .map_err(|e| AgentFlowError::Other(anyhow!("Request failed: {}", e)))?;
    
    let categories = response["categories"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|v| {
            Some(SafetyCategory {
                name: v["name"].as_str()?.to_string(),
                score: v["score"].as_f64()? as f32,
                flagged: v["flagged"].as_bool().unwrap_or(false),
            })
        })
        .collect();

    Ok(ContentSafetyResponse {
        safe: response["safe"].as_bool().unwrap_or(true),
        categories,
        metadata: response,
    })
}

