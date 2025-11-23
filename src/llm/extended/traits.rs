use std::sync::Arc;
use async_trait::async_trait;
use crate::error::Result;

use super::types::*;

#[async_trait]
pub trait ExtendedApiClient: Send + Sync {
    fn base_url(&self) -> &str;
    fn api_key(&self) -> &str;
    
    async fn chat_completion(&self, request: ChatCompletionRequest) -> Result<ChatCompletionResponse>;
    async fn chat_completion_async(&self, request: ChatCompletionRequest) -> Result<AsyncTaskResponse>;
    async fn get_async_result(&self, task_id: &str) -> Result<AsyncTaskResult>;
    
    async fn generate_image(&self, request: ImageGenerationRequest) -> Result<ImageGenerationResponse>;
    async fn generate_video_async(&self, request: VideoGenerationRequest) -> Result<AsyncTaskResponse>;
    
    async fn speech_to_text(&self, request: SpeechToTextRequest) -> Result<SpeechToTextResponse>;
    async fn text_to_speech(&self, request: TextToSpeechRequest) -> Result<TextToSpeechResponse>;
    async fn clone_voice(&self, request: VoiceCloneRequest) -> Result<VoiceCloneResponse>;
    async fn list_voices(&self) -> Result<VoiceListResponse>;
    async fn delete_voice(&self, voice_id: &str) -> Result<()>;
    
    async fn text_embedding(&self, request: TextEmbeddingRequest) -> Result<TextEmbeddingResponse>;
    async fn text_rerank(&self, request: TextRerankRequest) -> Result<TextRerankResponse>;
    async fn text_tokenize(&self, request: TextTokenizeRequest) -> Result<TextTokenizeResponse>;
    
    async fn web_search(&self, request: WebSearchRequest) -> Result<WebSearchResponse>;
    async fn web_read(&self, request: WebReadRequest) -> Result<WebReadResponse>;
    async fn content_safety(&self, request: ContentSafetyRequest) -> Result<ContentSafetyResponse>;
    
    async fn parse_file_sync(&self, request: FileParseRequest) -> Result<FileParseResponse>;
    async fn parse_file_async(&self, request: FileParseRequest) -> Result<AsyncTaskResponse>;
    
    async fn agent_chat(&self, request: AgentChatRequest) -> Result<AgentChatResponse>;
    async fn agent_chat_async(&self, request: AgentChatRequest) -> Result<AsyncTaskResponse>;
    async fn agent_history(&self, session_id: &str) -> Result<AgentHistoryResponse>;
    
    async fn list_files(&self) -> Result<FileListResponse>;
    async fn upload_file(&self, request: FileUploadRequest) -> Result<FileUploadResponse>;
    async fn delete_file(&self, file_id: &str) -> Result<()>;
    async fn get_file_content(&self, file_id: &str) -> Result<FileContentResponse>;
    
    async fn list_batch_tasks(&self) -> Result<BatchTaskListResponse>;
    async fn create_batch_task(&self, request: BatchTaskRequest) -> Result<BatchTaskResponse>;
    async fn get_batch_task(&self, task_id: &str) -> Result<BatchTaskResponse>;
    async fn cancel_batch_task(&self, task_id: &str) -> Result<()>;
    
    async fn knowledge_retrieve(&self, request: KnowledgeRetrieveRequest) -> Result<KnowledgeRetrieveResponse>;
    async fn list_knowledge_bases(&self) -> Result<KnowledgeBaseListResponse>;
    async fn create_knowledge_base(&self, request: KnowledgeBaseCreateRequest) -> Result<KnowledgeBaseResponse>;
    async fn get_knowledge_base(&self, kb_id: &str) -> Result<KnowledgeBaseResponse>;
    async fn update_knowledge_base(&self, kb_id: &str, request: KnowledgeBaseUpdateRequest) -> Result<KnowledgeBaseResponse>;
    async fn delete_knowledge_base(&self, kb_id: &str) -> Result<()>;
    async fn get_knowledge_base_usage(&self, kb_id: &str) -> Result<KnowledgeBaseUsageResponse>;
    async fn list_documents(&self, kb_id: &str) -> Result<DocumentListResponse>;
    async fn upload_document_file(&self, kb_id: &str, request: DocumentUploadRequest) -> Result<DocumentResponse>;
    async fn upload_document_url(&self, kb_id: &str, request: DocumentUrlRequest) -> Result<DocumentResponse>;
    async fn parse_document_image(&self, kb_id: &str, request: DocumentImageParseRequest) -> Result<DocumentResponse>;
    async fn get_document(&self, kb_id: &str, doc_id: &str) -> Result<DocumentResponse>;
    async fn delete_document(&self, kb_id: &str, doc_id: &str) -> Result<()>;
    async fn reindex_document(&self, kb_id: &str, doc_id: &str) -> Result<()>;
    
    async fn assistant_chat(&self, request: AssistantChatRequest) -> Result<AssistantChatResponse>;
}

pub type DynExtendedApiClient = Arc<dyn ExtendedApiClient>;

