//! 文件下载工具 - 支持图片、音频、视频等（内置工具）

use async_trait::async_trait;
use reqwest::Client;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::agent::{AgentMessage, MessageRole};
use crate::error::{AgentFlowError, Result};
use crate::state::FlowContext;
use crate::tools::tool::{Tool, ToolInvocation};

/// 文件下载工具
/// 
/// 这是一个内置工具，可以自动从消息历史中提取 URL 并下载文件
/// 
/// 输入参数（可选）：
/// - url: 直接指定要下载的 URL（不提供时自动从上下文提取）
/// - save_dir: 保存目录（默认 "downloads"，项目根目录下）
/// - filename_prefix: 文件名前缀（默认 "file"）
///
/// 示例配置：
/// ```json
/// {
///   "save_dir": "output/images",     // 自定义目录
///   "filename_prefix": "marketing"   // 自定义前缀
/// }
/// ```
#[derive(Clone)]
pub struct DownloaderTool {
    client: Client,
}

impl Default for DownloaderTool {
    fn default() -> Self {
        Self::new()
    }
}

impl DownloaderTool {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// 从消息历史中提取 URL
    fn extract_url_from_context(ctx: &FlowContext) -> Option<String> {
        let history = ctx.history();
        
        for msg in history.iter().rev() {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&msg.content) {
                let url_candidates = vec!["image_url", "url", "file_url", "download_url"];
                
                if let Some(response_str) = json["response"].as_str() {
                    if let Ok(response_json) = serde_json::from_str::<serde_json::Value>(response_str) {
                        for key in &url_candidates {
                            if let Some(url) = response_json[key].as_str() {
                                if !url.is_empty() && (url.starts_with("http://") || url.starts_with("https://")) {
                                    return Some(url.to_string());
                                }
                            }
                        }
                    }
                }
                
                for key in &url_candidates {
                    if let Some(url) = json[key].as_str() {
                        if !url.is_empty() && (url.starts_with("http://") || url.starts_with("https://")) {
                            return Some(url.to_string());
                        }
                    }
                }
            }
        }
        
        None
    }

    fn infer_extension(url: &str, content_type: Option<&str>) -> String {
        if let Some(ct) = content_type {
            let ext = match ct {
                // 图片
                ct if ct.contains("image/png") => "png",
                ct if ct.contains("image/jpeg") || ct.contains("image/jpg") => "jpg",
                ct if ct.contains("image/gif") => "gif",
                ct if ct.contains("image/webp") => "webp",
                ct if ct.contains("image/svg") => "svg",
                // 音频
                ct if ct.contains("audio/mpeg") || ct.contains("audio/mp3") => "mp3",
                ct if ct.contains("audio/wav") => "wav",
                ct if ct.contains("audio/ogg") => "ogg",
                ct if ct.contains("audio/aac") => "aac",
                // 视频
                ct if ct.contains("video/mp4") => "mp4",
                ct if ct.contains("video/mpeg") => "mpeg",
                ct if ct.contains("video/quicktime") => "mov",
                ct if ct.contains("video/x-msvideo") => "avi",
                ct if ct.contains("video/webm") => "webm",
                // 文档
                ct if ct.contains("application/pdf") => "pdf",
                ct if ct.contains("application/json") => "json",
                ct if ct.contains("text/plain") => "txt",
                _ => "bin",
            };
            return ext.to_string();
        }

        if let Some(ext) = url.rsplit('.').next() {
            if ext.len() <= 5 && !ext.contains('/') && !ext.contains('?') {
                if let Some(clean_ext) = ext.split('?').next() {
                    return clean_ext.to_string();
                }
            }
        }

        "bin".to_string()
    }
}

#[async_trait]
impl Tool for DownloaderTool {
    fn name(&self) -> &'static str {
        "downloader"
    }

    async fn call(&self, invocation: ToolInvocation, ctx: &FlowContext) -> Result<AgentMessage> {
        let url = invocation.input["url"]
            .as_str()
            .map(|s| s.to_string())
            .or_else(|| Self::extract_url_from_context(ctx))
            .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("No URL found in input or context")))?;

        let save_dir = invocation.input["save_dir"]
            .as_str()
            .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Missing save_dir")))?;

        let filename_prefix = invocation.input["filename_prefix"]
            .as_str()
            .ok_or_else(|| AgentFlowError::Other(anyhow::anyhow!("Missing filename_prefix")))?;

        println!("🖼️  检测到文件，正在下载...");
        println!("URL: {}", url);

        fs::create_dir_all(save_dir).map_err(|e| {
            AgentFlowError::Other(anyhow::anyhow!("Failed to create directory: {}", e))
        })?;

        let response = self.client.get(&url).send().await.map_err(|e| {
            AgentFlowError::Other(anyhow::anyhow!("Download request failed: {}", e))
        })?;

        if !response.status().is_success() {
            return Err(AgentFlowError::Other(anyhow::anyhow!(
                "Download failed with HTTP status: {}",
                response.status()
            )));
        }

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let extension = Self::infer_extension(&url, content_type.as_deref());

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| AgentFlowError::Other(anyhow::anyhow!("Time error: {}", e)))?
            .as_secs();

        let filename = format!("{}_{}.{}", filename_prefix, timestamp, extension);
        let filepath = format!("{}/{}", save_dir, filename);

        let bytes = response.bytes().await.map_err(|e| {
            AgentFlowError::Other(anyhow::anyhow!("Failed to read response: {}", e))
        })?;

        let file_size = bytes.len();

        fs::write(&filepath, bytes).map_err(|e| {
            AgentFlowError::Other(anyhow::anyhow!("Failed to write file: {}", e))
        })?;

        println!("✅ 文件已保存到: {}", filepath);

        let result = serde_json::json!({
            "success": true,
            "url": url,
            "filepath": filepath,
            "filename": filename,
            "size_bytes": file_size,
            "extension": extension,
            "content_type": content_type
        });

        Ok(AgentMessage {
            id: crate::agent::message::uuid(),
            role: MessageRole::Tool,
            from: self.name().to_string(),
            to: None,
            content: result.to_string(),
            metadata: None,
        })
    }
}
