use serde_json::json;
use crate::error::{Result, AgentFlowError};
use anyhow::anyhow;
use super::core::GenericApiClient;
use super::super::types::*;

/// Web 相关 API 实现

pub async fn web_search(
    client: &GenericApiClient,
    request: WebSearchRequest,
) -> Result<WebSearchResponse> {
    let body = json!({
        "query": request.query,
        "max_results": request.max_results,
    });

    let response = client.request("POST", "web_search", Some(body)).await
        .map_err(|e| AgentFlowError::Other(anyhow!("Request failed: {}", e)))?;
    
    let results = response["results"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|v| {
            Some(SearchResult {
                title: v["title"].as_str()?.to_string(),
                url: v["url"].as_str()?.to_string(),
                snippet: v["snippet"].as_str()?.to_string(),
                metadata: v.clone(),
            })
        })
        .collect();

    Ok(WebSearchResponse {
        results,
        metadata: response,
    })
}

pub async fn web_read(
    client: &GenericApiClient,
    request: WebReadRequest,
) -> Result<WebReadResponse> {
    let body = json!({
        "url": request.url,
    });

    let response = client.request("POST", "web_read", Some(body)).await
        .map_err(|e| AgentFlowError::Other(anyhow!("Request failed: {}", e)))?;
    
    Ok(WebReadResponse {
        content: response["content"].as_str()
            .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Missing content")))?
            .to_string(),
        title: response["title"].as_str().map(|s| s.to_string()),
        metadata: response,
    })
}

