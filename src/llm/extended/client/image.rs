use serde_json::json;
use crate::error::{Result, AgentFlowError};
use anyhow::anyhow;
use super::core::GenericApiClient;
use super::super::types::*;

/// 图像相关 API 实现

pub async fn generate_image(
    client: &GenericApiClient,
    request: ImageGenerationRequest,
) -> Result<ImageGenerationResponse> {
    let body = json!({
        "model": request.model,
        "prompt": request.prompt,
        "size": request.size,
        "quality": request.quality,
        "n": request.n,
    });

    let response = client.request("POST", "generate_image", Some(body)).await
        .map_err(|e| AgentFlowError::Other(anyhow!("Request failed: {}", e)))?;
    
    let images = response["images"]
        .as_array()
        .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Missing images array")))?
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();

    Ok(ImageGenerationResponse {
        images,
        metadata: response,
    })
}

pub async fn generate_video_async(
    client: &GenericApiClient,
    request: VideoGenerationRequest,
) -> Result<AsyncTaskResponse> {
    let body = json!({
        "model": request.model,
        "prompt": request.prompt,
        "duration": request.duration,
    });

    let response = client.request("POST", "generate_video_async", Some(body)).await
        .map_err(|e| AgentFlowError::Other(anyhow!("Request failed: {}", e)))?;
    
    Ok(AsyncTaskResponse {
        task_id: response["task_id"].as_str()
            .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Missing task_id")))?
            .to_string(),
        status: response["status"].as_str().unwrap_or("pending").to_string(),
        metadata: response,
    })
}

