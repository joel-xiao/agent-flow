// GenericApiClient 模块 - 按功能域拆分

mod core;
mod chat;
mod image;
mod voice;
mod text;
mod web;
mod safety;
mod file;
mod batch;
mod knowledge;
mod document;
mod agent;
mod assistant;

pub use core::GenericApiClient;

// 实现 ExtendedApiClient trait
use super::traits::ExtendedApiClient;
use super::types::*;
use crate::error::Result;

// 将 trait 实现放在主模块中，但方法实现分散到各个子模块
#[async_trait::async_trait]
impl ExtendedApiClient for GenericApiClient {
    fn base_url(&self) -> &str {
        core::GenericApiClient::base_url(self)
    }

    fn api_key(&self) -> &str {
        &self.api_key
    }

    // Chat 相关方法
    async fn chat_completion(&self, request: ChatCompletionRequest) -> Result<ChatCompletionResponse> {
        chat::chat_completion(self, request).await
    }

    async fn chat_completion_async(&self, request: ChatCompletionRequest) -> Result<AsyncTaskResponse> {
        chat::chat_completion_async(self, request).await
    }

    async fn get_async_result(&self, task_id: &str) -> Result<AsyncTaskResult> {
        chat::get_async_result(self, task_id).await
    }

    // 图像相关方法
    async fn generate_image(&self, request: ImageGenerationRequest) -> Result<ImageGenerationResponse> {
        image::generate_image(self, request).await
    }

    async fn generate_video_async(&self, request: VideoGenerationRequest) -> Result<AsyncTaskResponse> {
        image::generate_video_async(self, request).await
    }

    // 语音相关方法
    async fn speech_to_text(&self, request: SpeechToTextRequest) -> Result<SpeechToTextResponse> {
        voice::speech_to_text(self, request).await
    }

    async fn text_to_speech(&self, request: TextToSpeechRequest) -> Result<TextToSpeechResponse> {
        voice::text_to_speech(self, request).await
    }

    async fn clone_voice(&self, request: VoiceCloneRequest) -> Result<VoiceCloneResponse> {
        voice::clone_voice(self, request).await
    }

    async fn list_voices(&self) -> Result<VoiceListResponse> {
        voice::list_voices(self).await
    }

    async fn delete_voice(&self, voice_id: &str) -> Result<()> {
        voice::delete_voice(self, voice_id).await
    }

    // 文本处理相关方法
    async fn text_embedding(&self, request: TextEmbeddingRequest) -> Result<TextEmbeddingResponse> {
        text::text_embedding(self, request).await
    }

    async fn text_rerank(&self, request: TextRerankRequest) -> Result<TextRerankResponse> {
        text::text_rerank(self, request).await
    }

    async fn text_tokenize(&self, request: TextTokenizeRequest) -> Result<TextTokenizeResponse> {
        text::text_tokenize(self, request).await
    }

    // Web 相关方法
    async fn web_search(&self, request: WebSearchRequest) -> Result<WebSearchResponse> {
        web::web_search(self, request).await
    }

    async fn web_read(&self, request: WebReadRequest) -> Result<WebReadResponse> {
        web::web_read(self, request).await
    }

    // 内容安全相关方法
    async fn content_safety(&self, request: ContentSafetyRequest) -> Result<ContentSafetyResponse> {
        safety::content_safety(self, request).await
    }

    // 文件相关方法
    async fn parse_file_sync(&self, request: FileParseRequest) -> Result<FileParseResponse> {
        file::parse_file_sync(self, request).await
    }

    async fn parse_file_async(&self, request: FileParseRequest) -> Result<AsyncTaskResponse> {
        file::parse_file_async(self, request).await
    }

    async fn list_files(&self) -> Result<FileListResponse> {
        file::list_files(self).await
    }

    async fn upload_file(&self, request: FileUploadRequest) -> Result<FileUploadResponse> {
        file::upload_file(self, request).await
    }

    async fn delete_file(&self, file_id: &str) -> Result<()> {
        file::delete_file(self, file_id).await
    }

    async fn get_file_content(&self, file_id: &str) -> Result<FileContentResponse> {
        file::get_file_content(self, file_id).await
    }

    // 批处理相关方法
    async fn list_batch_tasks(&self) -> Result<BatchTaskListResponse> {
        batch::list_batch_tasks(self).await
    }

    async fn create_batch_task(&self, request: BatchTaskRequest) -> Result<BatchTaskResponse> {
        batch::create_batch_task(self, request).await
    }

    async fn get_batch_task(&self, task_id: &str) -> Result<BatchTaskResponse> {
        batch::get_batch_task(self, task_id).await
    }

    async fn cancel_batch_task(&self, task_id: &str) -> Result<()> {
        batch::cancel_batch_task(self, task_id).await
    }

    // 知识库相关方法
    async fn knowledge_retrieve(&self, request: KnowledgeRetrieveRequest) -> Result<KnowledgeRetrieveResponse> {
        knowledge::knowledge_retrieve(self, request).await
    }

    async fn list_knowledge_bases(&self) -> Result<KnowledgeBaseListResponse> {
        knowledge::list_knowledge_bases(self).await
    }

    async fn create_knowledge_base(&self, request: KnowledgeBaseCreateRequest) -> Result<KnowledgeBaseResponse> {
        knowledge::create_knowledge_base(self, request).await
    }

    async fn get_knowledge_base(&self, kb_id: &str) -> Result<KnowledgeBaseResponse> {
        knowledge::get_knowledge_base(self, kb_id).await
    }

    async fn update_knowledge_base(&self, kb_id: &str, request: KnowledgeBaseUpdateRequest) -> Result<KnowledgeBaseResponse> {
        knowledge::update_knowledge_base(self, kb_id, request).await
    }

    async fn delete_knowledge_base(&self, kb_id: &str) -> Result<()> {
        knowledge::delete_knowledge_base(self, kb_id).await
    }

    async fn get_knowledge_base_usage(&self, kb_id: &str) -> Result<KnowledgeBaseUsageResponse> {
        knowledge::get_knowledge_base_usage(self, kb_id).await
    }

    // 文档相关方法
    async fn list_documents(&self, kb_id: &str) -> Result<DocumentListResponse> {
        document::list_documents(self, kb_id).await
    }

    async fn upload_document_file(&self, kb_id: &str, request: DocumentUploadRequest) -> Result<DocumentResponse> {
        document::upload_document_file(self, kb_id, request).await
    }

    async fn upload_document_url(&self, kb_id: &str, request: DocumentUrlRequest) -> Result<DocumentResponse> {
        document::upload_document_url(self, kb_id, request).await
    }

    async fn parse_document_image(&self, kb_id: &str, request: DocumentImageParseRequest) -> Result<DocumentResponse> {
        document::parse_document_image(self, kb_id, request).await
    }

    async fn get_document(&self, kb_id: &str, doc_id: &str) -> Result<DocumentResponse> {
        document::get_document(self, kb_id, doc_id).await
    }

    async fn delete_document(&self, kb_id: &str, doc_id: &str) -> Result<()> {
        document::delete_document(self, kb_id, doc_id).await
    }

    async fn reindex_document(&self, kb_id: &str, doc_id: &str) -> Result<()> {
        document::reindex_document(self, kb_id, doc_id).await
    }

    // Agent 相关方法
    async fn agent_chat(&self, request: AgentChatRequest) -> Result<AgentChatResponse> {
        agent::agent_chat(self, request).await
    }

    async fn agent_chat_async(&self, request: AgentChatRequest) -> Result<AsyncTaskResponse> {
        agent::agent_chat_async(self, request).await
    }

    async fn agent_history(&self, session_id: &str) -> Result<AgentHistoryResponse> {
        agent::agent_history(self, session_id).await
    }

    // Assistant 相关方法
    async fn assistant_chat(&self, request: AssistantChatRequest) -> Result<AssistantChatResponse> {
        assistant::assistant_chat(self, request).await
    }
}

