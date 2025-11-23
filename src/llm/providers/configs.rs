use crate::llm::config::ApiEndpointConfig;

pub fn bigmodel_config(base_url: Option<&str>) -> ApiEndpointConfig {
    let base = base_url.unwrap_or("https://open.bigmodel.cn/api/paas/v4");
    ApiEndpointConfig::new(base)
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
        .with_endpoint("agent_chat", "/agents/chat")
        .with_endpoint("agent_chat_async", "/agents/chat/async")
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
        .with_default_header("Accept", "application/json")
        .with_default_header("User-Agent", "agentflow/1.0.0")
}

pub fn qwen_config(base_url: Option<&str>) -> ApiEndpointConfig {
    let base = base_url.unwrap_or("https://dashscope.aliyuncs.com/api/v1");
    ApiEndpointConfig::new(base)
        .with_endpoint("chat_completion", "/services/aigc/text-generation/generation")
        .with_endpoint("chat_completion_async", "/services/aigc/text-generation/async")
        .with_endpoint("generate_image", "/services/aigc/image-generation/generation")
        .with_endpoint("text_embedding", "/services/embeddings/text-embedding/text-embedding")
        .with_endpoint("speech_to_text", "/services/audio/asr/transcription")
        .with_endpoint("text_to_speech", "/services/audio/tts/text-to-speech")
}

pub fn moonshot_config(base_url: Option<&str>) -> ApiEndpointConfig {
    let base = base_url.unwrap_or("https://api.moonshot.cn/v1");
    ApiEndpointConfig::new(base)
        .with_endpoint("chat_completion", "/chat/completions")
        .with_endpoint("text_embedding", "/embeddings")
}

pub fn openai_config(base_url: Option<&str>) -> ApiEndpointConfig {
    let base = base_url.unwrap_or("https://api.openai.com/v1");
    ApiEndpointConfig::new(base)
        .with_endpoint("chat_completion", "/chat/completions")
        .with_endpoint("chat_completion_async", "/chat/completions")
        .with_endpoint("generate_image", "/images/generations")
        .with_endpoint("text_embedding", "/embeddings")
        .with_endpoint("speech_to_text", "/audio/transcriptions")
        .with_endpoint("text_to_speech", "/audio/speech")
        .with_endpoint("list_files", "/files")
        .with_endpoint("upload_file", "/files")
        .with_endpoint("delete_file", "/files/{file_id}")
        .with_endpoint("get_file_content", "/files/{file_id}/content")
}

pub fn deepseek_config(base_url: Option<&str>) -> ApiEndpointConfig {
    let base = base_url.unwrap_or("https://api.deepseek.com/v1");
    ApiEndpointConfig::new(base)
        .with_endpoint("chat_completion", "/chat/completions")
        .with_endpoint("text_embedding", "/embeddings")
}

pub fn generic_config(base_url: &str) -> ApiEndpointConfig {
    ApiEndpointConfig::new(base_url)
        .with_endpoint("chat_completion", "/chat/completions")
        .with_endpoint("text_embedding", "/embeddings")
}

