use serde_json::json;
use crate::error::{Result, AgentFlowError};
use anyhow::anyhow;
use super::core::GenericApiClient;
use super::super::types::*;

/// 文本处理相关 API 实现

pub async fn text_embedding(
    client: &GenericApiClient,
    request: TextEmbeddingRequest,
) -> Result<TextEmbeddingResponse> {
    let body = json!({
        "model": request.model,
        "texts": request.texts,
    });

    let response = client.request("POST", "text_embedding", Some(body)).await
        .map_err(|e| AgentFlowError::Other(anyhow!("Request failed: {}", e)))?;
    
    let embeddings = response["embeddings"]
        .as_array()
        .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Missing embeddings")))?
        .iter()
        .filter_map(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_f64().map(|f| f as f32)).collect())
        .collect();

    Ok(TextEmbeddingResponse {
        embeddings,
        metadata: response,
    })
}

pub async fn text_rerank(
    client: &GenericApiClient,
    request: TextRerankRequest,
) -> Result<TextRerankResponse> {
    let body = json!({
        "model": request.model,
        "query": request.query,
        "documents": request.documents,
        "top_k": request.top_k,
    });

    let response = client.request("POST", "text_rerank", Some(body)).await
        .map_err(|e| AgentFlowError::Other(anyhow!("Request failed: {}", e)))?;
    
    let results = response["results"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|v| {
            Some(RerankResult {
                index: v["index"].as_u64()? as usize,
                score: v["score"].as_f64()? as f32,
                text: v["text"].as_str()?.to_string(),
            })
        })
        .collect();

    Ok(TextRerankResponse {
        results,
        metadata: response,
    })
}

pub async fn text_tokenize(
    client: &GenericApiClient,
    request: TextTokenizeRequest,
) -> Result<TextTokenizeResponse> {
    let body = json!({
        "model": request.model,
        "text": request.text,
    });

    let response = client.request("POST", "text_tokenize", Some(body)).await
        .map_err(|e| AgentFlowError::Other(anyhow!("Request failed: {}", e)))?;
    
    Ok(TextTokenizeResponse {
        tokens: response["tokens"].as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
        token_ids: response["token_ids"].as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| v.as_u64().map(|u| u as u32))
            .collect(),
        metadata: response,
    })
}

