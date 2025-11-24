//! 图片生成工具 - 调用通义万相 API

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use tokio::time::{sleep, Duration};

use crate::agent::{AgentMessage, MessageRole};
use crate::error::{AgentFlowError, Result};
use crate::state::FlowContext;
use crate::tools::tool::{Tool, ToolInvocation};

/// 图片生成工具
pub struct ImageGeneratorTool {
    client: Client,
}

impl Default for ImageGeneratorTool {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageGeneratorTool {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct WanxiangResponse {
    output: WanxiangOutput,
}

#[derive(Debug, Deserialize)]
struct WanxiangOutput {
    task_id: String,
    task_status: String,
    #[serde(default)]
    results: Vec<WanxiangResult>,
}

#[derive(Debug, Deserialize)]
struct WanxiangResult {
    url: String,
}

#[async_trait]
impl Tool for ImageGeneratorTool {
    fn name(&self) -> &'static str {
        "image_generator"
    }

    async fn call(&self, invocation: ToolInvocation, _ctx: &FlowContext) -> Result<AgentMessage> {
        let api_key = invocation.input["api_key"]
            .as_str()
            .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Missing api_key")))?;
        
        let prompt = invocation.input["prompt"]
            .as_str()
            .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Missing prompt")))?;
        
        let model = invocation.input["model"]
            .as_str()
            .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Missing model parameter")))?;
        
        let endpoint = invocation.input["endpoint"]
            .as_str()
            .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Missing endpoint parameter")))?;
        
        let query_endpoint = invocation.input["query_endpoint"]
            .as_str()
            .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Missing query_endpoint parameter")))?;
        
        let style = invocation.input["style"]
            .as_str()
            .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Missing style parameter")))?;
        
        let size = invocation.input["size"]
            .as_str()
            .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Missing size parameter")))?;

        let req_body = json!({
            "model": model,
            "input": {
                "prompt": prompt
            },
            "parameters": {
                "style": style,
                "size": size,
                "n": 1
            }
        });

        let response = self.client
            .post(endpoint)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .header("X-DashScope-Async", "enable")
            .json(&req_body)
            .send()
            .await
            .map_err(|e| AgentFlowError::Other(anyhow::anyhow!("API request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await
                .map_err(|e| AgentFlowError::Other(anyhow::anyhow!("Failed to read error response: {}", e)))?;
            return Err(AgentFlowError::Other(anyhow::anyhow!(
                "API error: {}",
                error_text
            )));
        }

        let submit_resp: WanxiangResponse = response
            .json()
            .await
            .map_err(|e| AgentFlowError::Other(anyhow::anyhow!("Parse response failed: {}", e)))?;

        if submit_resp.output.task_status == "SUCCEEDED" && !submit_resp.output.results.is_empty() {
            let image_urls: Vec<String> = submit_resp
                .output
                .results
                .into_iter()
                .map(|r| r.url)
                .collect();

            return Ok(AgentMessage {
                id: crate::agent::message::uuid(),
                role: MessageRole::Tool,
                from: self.name().to_string(),
                to: None,
                content: json!({
                    "success": true,
                    "image_urls": image_urls,
                    "task_id": submit_resp.output.task_id
                })
                .to_string(),
                metadata: None,
            });
        }

        let task_id = submit_resp.output.task_id;
        let query_url = format!("{}/{}", query_endpoint, task_id);
        let max_attempts = 60;

        for attempt in 1..=max_attempts {
            sleep(Duration::from_secs(2)).await;

            let query_response = self.client
                .get(&query_url)
                .header("Authorization", format!("Bearer {}", api_key))
                .send()
                .await;

            if query_response.is_err() {
                continue;
            }

            let query_response = query_response.unwrap();
            if !query_response.status().is_success() {
                continue;
            }

            let result: WanxiangResponse = match query_response.json().await {
                Ok(r) => r,
                Err(_) => continue,
            };

            match result.output.task_status.as_str() {
                "SUCCEEDED" => {
                    let image_urls: Vec<String> = result
                        .output
                        .results
                        .into_iter()
                        .map(|r| r.url)
                        .collect();

                    return Ok(AgentMessage {
                        id: crate::agent::message::uuid(),
                        role: MessageRole::Tool,
                        from: self.name().to_string(),
                        to: None,
                        content: json!({
                            "success": true,
                            "image_urls": image_urls,
                            "task_id": task_id,
                            "attempts": attempt
                        })
                        .to_string(),
                        metadata: None,
                    });
                }
                "FAILED" => {
                    return Err(AgentFlowError::Other(anyhow::anyhow!(
                        "Image generation failed"
                    )));
                }
                "RUNNING" | "PENDING" => {
                    continue;
                }
                _ => continue,
            }
        }

        Err(AgentFlowError::Other(anyhow::anyhow!(
            "Image generation timeout after {} attempts",
            max_attempts
        )))
    }
}

