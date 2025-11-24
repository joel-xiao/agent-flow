use crate::error::{AgentFlowError, Result};
use crate::llm::types::LlmStreamChunk;
use anyhow::anyhow;
use serde_json::Value;

/// SSE (Server-Sent Events) 解析器
///
/// 用于解析流式响应中的 SSE 格式数据
pub struct SseParser {
    buffer: String,
}

impl SseParser {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    /// 解析数据块，返回流式 chunk 列表
    ///
    /// SSE 格式：
    /// ```
    /// data: {"id":"...","choices":[{"delta":{"content":"Hello"}}]}
    ///
    /// data: {"id":"...","choices":[{"delta":{"content":" world"}}]}
    ///
    /// data: [DONE]
    /// ```
    pub fn parse_chunk(&mut self, data: &[u8]) -> Result<Vec<LlmStreamChunk>> {
        let text = String::from_utf8_lossy(data);
        self.buffer.push_str(&text);

        let mut chunks = Vec::new();
        let mut processed = 0;

        while let Some(end_pos) = self.buffer[processed..].find("\n\n") {
            let event_end = processed + end_pos;
            let event_text = &self.buffer[processed..event_end];

            if let Some(chunk) = self.parse_event(event_text)? {
                chunks.push(chunk);
            }

            processed = event_end + 2;
        }

        if processed > 0 {
            self.buffer.drain(..processed);
        }

        Ok(chunks)
    }

    /// 解析单个 SSE 事件
    fn parse_event(&self, event_text: &str) -> Result<Option<LlmStreamChunk>> {
        let data = if event_text.starts_with("data: ") {
            &event_text[6..]
        } else if event_text.starts_with("data:") {
            &event_text[5..].trim_start()
        } else {
            event_text.trim()
        };

        if data.trim() == "[DONE]" {
            return Ok(Some(LlmStreamChunk {
                content: String::new(),
                done: true,
            }));
        }

        let json: Value = serde_json::from_str(data).map_err(|e| {
            AgentFlowError::Other(anyhow!("Failed to parse SSE JSON: {}: {}", e, data))
        })?;

        let content = self.extract_content_delta(&json)?;

        if content.is_empty() {
            Ok(None)
        } else {
            Ok(Some(LlmStreamChunk {
                content,
                done: false,
            }))
        }
    }

    /// 从 JSON 中提取 content delta
    ///
    /// 支持多种 API 格式：
    /// - OpenAI: choices[0].delta.content
    /// - Qwen: output.text 或 choices[0].delta.content
    /// - QwenVision: choices[0].delta.content
    fn extract_content_delta(&self, json: &Value) -> Result<String> {
        if let Some(content) = json["choices"]
            .as_array()
            .and_then(|choices| choices.get(0))
            .and_then(|choice| choice["delta"]["content"].as_str())
        {
            return Ok(content.to_string());
        }

        if let Some(content) = json["output"]["text"].as_str() {
            return Ok(content.to_string());
        }

        if let Some(content) = json["output"]["choices"]
            .as_array()
            .and_then(|choices| choices.get(0))
            .and_then(|choice| choice["message"]["content"].as_str())
        {
            return Ok(content.to_string());
        }

        Ok(String::new())
    }

    /// 清空 buffer
    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

impl Default for SseParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_openai_sse() {
        let mut parser = SseParser::new();
        let data = b"data: {\"id\":\"chatcmpl-123\",\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}\n\ndata: {\"id\":\"chatcmpl-123\",\"choices\":[{\"delta\":{\"content\":\" world\"}}]}\n\n";

        let chunks = parser.parse_chunk(data).unwrap();
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].content, "Hello");
        assert_eq!(chunks[1].content, " world");
    }

    #[test]
    fn test_parse_done() {
        let mut parser = SseParser::new();
        let data = b"data: [DONE]\n\n";

        let chunks = parser.parse_chunk(data).unwrap();
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].done);
    }

    #[test]
    fn test_parse_qwen_format() {
        let mut parser = SseParser::new();
        let data = b"data: {\"output\":{\"text\":\"Hello\"}}\n\n";

        let chunks = parser.parse_chunk(data).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content, "Hello");
    }
}
