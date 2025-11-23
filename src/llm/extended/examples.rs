use super::universal::{UniversalApiClient, ApiBuilder};
use crate::llm::config::ApiEndpointConfig;
use serde_json::json;

pub fn create_bigmodel_client(api_key: &str) -> ApiBuilder {
    let config = ApiEndpointConfig::new("https://open.bigmodel.cn/api/paas/v4")
        .with_endpoint("chat_completion", "/chat/completions")
        .with_endpoint("chat_completion_async", "/chat/completions/async")
        .with_endpoint("get_async_result", "/async/{task_id}")
        .with_endpoint("generate_image", "/images/generations")
        .with_endpoint("generate_video_async", "/videos/generations/async")
        .with_endpoint("speech_to_text", "/audio/transcriptions")
        .with_endpoint("text_to_speech", "/audio/speech")
        .with_endpoint("clone_voice", "/audio/voices")
        .with_endpoint("list_voices", "/audio/voices")
        .with_endpoint("delete_voice", "/audio/voices/{voice_id}")
        .with_endpoint("text_embedding", "/embeddings")
        .with_endpoint("text_rerank", "/rerank")
        .with_endpoint("text_tokenize", "/tokenize")
        .with_endpoint("web_search", "/tools/web_search")
        .with_endpoint("web_read", "/tools/web_read")
        .with_endpoint("content_safety", "/moderations")
        .with_endpoint("parse_file_sync", "/files/parse")
        .with_endpoint("parse_file_async", "/files/parse/async")
        .with_endpoint("get_parse_result", "/files/parse/{task_id}")
        .with_endpoint("agent_chat", "/agents/chat")
        .with_endpoint("agent_chat_async", "/agents/chat/async")
        .with_endpoint("get_agent_async_result", "/agents/chat/async/{task_id}")
        .with_endpoint("agent_history", "/agents/sessions/{session_id}/history")
        .with_endpoint("list_files", "/files")
        .with_endpoint("upload_file", "/files")
        .with_endpoint("delete_file", "/files/{file_id}")
        .with_endpoint("get_file_content", "/files/{file_id}/content")
        .with_endpoint("list_batch_tasks", "/batch")
        .with_endpoint("create_batch_task", "/batch")
        .with_endpoint("get_batch_task", "/batch/{task_id}")
        .with_endpoint("cancel_batch_task", "/batch/{task_id}/cancel")
        .with_endpoint("knowledge_retrieve", "/knowledge/retrieve")
        .with_endpoint("list_knowledge_bases", "/knowledge")
        .with_endpoint("create_knowledge_base", "/knowledge")
        .with_endpoint("get_knowledge_base", "/knowledge/{kb_id}")
        .with_endpoint("update_knowledge_base", "/knowledge/{kb_id}")
        .with_endpoint("delete_knowledge_base", "/knowledge/{kb_id}")
        .with_endpoint("get_knowledge_base_usage", "/knowledge/{kb_id}/usage")
        .with_endpoint("list_documents", "/knowledge/{kb_id}/documents")
        .with_endpoint("upload_document_file", "/knowledge/{kb_id}/documents/upload")
        .with_endpoint("upload_document_url", "/knowledge/{kb_id}/documents/url")
        .with_endpoint("parse_document_image", "/knowledge/{kb_id}/documents/image")
        .with_endpoint("get_document", "/knowledge/{kb_id}/documents/{doc_id}")
        .with_endpoint("delete_document", "/knowledge/{kb_id}/documents/{doc_id}")
        .with_endpoint("reindex_document", "/knowledge/{kb_id}/documents/{doc_id}/reindex")
        .with_endpoint("assistant_chat", "/assistants/chat")
        .with_endpoint("realtime_call", "/realtime/call")
        .with_default_header("Accept", "application/json")
        .with_default_header("User-Agent", "agentflow/1.0.0");

    let client = UniversalApiClient::new(config, api_key);
    ApiBuilder::new(client)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn example_chat_completion() {
        let api_key = std::env::var("BIGMODEL_API_KEY").unwrap_or_default();
        if api_key.is_empty() {
            return;
        }

        let builder = create_bigmodel_client(&api_key);
        let response = builder
            .post("chat_completion")
            .body(json!({
                "model": "glm-4",
                "messages": [
                    {"role": "user", "content": "Hello"}
                ],
                "temperature": 0.7,
                "max_tokens": 2000
            }))
            .call()
            .await
            .unwrap();

        println!("Response: {}", serde_json::to_string_pretty(&response).unwrap());
    }

    #[tokio::test]
    #[ignore]
    async fn example_get_async_result() {
        let api_key = std::env::var("BIGMODEL_API_KEY").unwrap_or_default();
        if api_key.is_empty() {
            return;
        }

        let builder = create_bigmodel_client(&api_key);
        let response = builder
            .get("get_async_result")
            .path_param("task_id", "test_task_id")
            .call()
            .await
            .unwrap();

        println!("Response: {}", serde_json::to_string_pretty(&response).unwrap());
    }
}

