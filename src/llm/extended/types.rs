use serde_json::Value;

#[derive(Clone, Debug)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<Value>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub stream: Option<bool>,
    pub metadata: Option<Value>,
}

#[derive(Clone, Debug)]
pub struct ChatCompletionResponse {
    pub content: String,
    pub metadata: Value,
}

#[derive(Clone, Debug)]
pub struct AsyncTaskResponse {
    pub task_id: String,
    pub status: String,
    pub metadata: Value,
}

#[derive(Clone, Debug)]
pub struct AsyncTaskResult {
    pub task_id: String,
    pub status: String,
    pub result: Option<Value>,
    pub error: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ImageGenerationRequest {
    pub model: String,
    pub prompt: String,
    pub size: Option<String>,
    pub quality: Option<String>,
    pub n: Option<u32>,
    pub metadata: Option<Value>,
}

#[derive(Clone, Debug)]
pub struct ImageGenerationResponse {
    pub images: Vec<String>,
    pub metadata: Value,
}

#[derive(Clone, Debug)]
pub struct VideoGenerationRequest {
    pub model: String,
    pub prompt: String,
    pub duration: Option<u32>,
    pub metadata: Option<Value>,
}

#[derive(Clone, Debug)]
pub struct SpeechToTextRequest {
    pub model: String,
    pub audio: Vec<u8>,
    pub format: Option<String>,
    pub language: Option<String>,
}

#[derive(Clone, Debug)]
pub struct SpeechToTextResponse {
    pub text: String,
    pub metadata: Value,
}

#[derive(Clone, Debug)]
pub struct TextToSpeechRequest {
    pub model: String,
    pub text: String,
    pub voice: Option<String>,
    pub speed: Option<f32>,
    pub format: Option<String>,
}

#[derive(Clone, Debug)]
pub struct TextToSpeechResponse {
    pub audio: Vec<u8>,
    pub format: String,
    pub metadata: Value,
}

#[derive(Clone, Debug)]
pub struct VoiceCloneRequest {
    pub name: String,
    pub audio_samples: Vec<Vec<u8>>,
    pub description: Option<String>,
}

#[derive(Clone, Debug)]
pub struct VoiceCloneResponse {
    pub voice_id: String,
    pub metadata: Value,
}

#[derive(Clone, Debug)]
pub struct VoiceListResponse {
    pub voices: Vec<VoiceInfo>,
}

#[derive(Clone, Debug)]
pub struct VoiceInfo {
    pub id: String,
    pub name: String,
    pub metadata: Value,
}

#[derive(Clone, Debug)]
pub struct TextEmbeddingRequest {
    pub model: String,
    pub texts: Vec<String>,
    pub metadata: Option<Value>,
}

#[derive(Clone, Debug)]
pub struct TextEmbeddingResponse {
    pub embeddings: Vec<Vec<f32>>,
    pub metadata: Value,
}

#[derive(Clone, Debug)]
pub struct TextRerankRequest {
    pub model: String,
    pub query: String,
    pub documents: Vec<String>,
    pub top_k: Option<u32>,
}

#[derive(Clone, Debug)]
pub struct TextRerankResponse {
    pub results: Vec<RerankResult>,
    pub metadata: Value,
}

#[derive(Clone, Debug)]
pub struct RerankResult {
    pub index: usize,
    pub score: f32,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct TextTokenizeRequest {
    pub model: String,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct TextTokenizeResponse {
    pub tokens: Vec<String>,
    pub token_ids: Vec<u32>,
    pub metadata: Value,
}

#[derive(Clone, Debug)]
pub struct WebSearchRequest {
    pub query: String,
    pub max_results: Option<u32>,
    pub metadata: Option<Value>,
}

#[derive(Clone, Debug)]
pub struct WebSearchResponse {
    pub results: Vec<SearchResult>,
    pub metadata: Value,
}

#[derive(Clone, Debug)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub metadata: Value,
}

#[derive(Clone, Debug)]
pub struct WebReadRequest {
    pub url: String,
    pub metadata: Option<Value>,
}

#[derive(Clone, Debug)]
pub struct WebReadResponse {
    pub content: String,
    pub title: Option<String>,
    pub metadata: Value,
}

#[derive(Clone, Debug)]
pub struct ContentSafetyRequest {
    pub text: String,
    pub categories: Option<Vec<String>>,
}

#[derive(Clone, Debug)]
pub struct ContentSafetyResponse {
    pub safe: bool,
    pub categories: Vec<SafetyCategory>,
    pub metadata: Value,
}

#[derive(Clone, Debug)]
pub struct SafetyCategory {
    pub name: String,
    pub score: f32,
    pub flagged: bool,
}

#[derive(Clone, Debug)]
pub struct FileParseRequest {
    pub file_id: String,
    pub parse_type: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Clone, Debug)]
pub struct FileParseResponse {
    pub content: String,
    pub metadata: Value,
}

#[derive(Clone, Debug)]
pub struct AgentChatRequest {
    pub agent_id: String,
    pub message: String,
    pub session_id: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Clone, Debug)]
pub struct AgentChatResponse {
    pub response: String,
    pub session_id: String,
    pub metadata: Value,
}

#[derive(Clone, Debug)]
pub struct AgentHistoryResponse {
    pub messages: Vec<Value>,
    pub metadata: Value,
}

#[derive(Clone, Debug)]
pub struct FileListResponse {
    pub files: Vec<FileInfo>,
    pub metadata: Value,
}

#[derive(Clone, Debug)]
pub struct FileInfo {
    pub id: String,
    pub name: String,
    pub size: u64,
    pub mime_type: String,
    pub created_at: String,
    pub metadata: Value,
}

#[derive(Clone, Debug)]
pub struct FileUploadRequest {
    pub name: String,
    pub content: Vec<u8>,
    pub mime_type: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Clone, Debug)]
pub struct FileUploadResponse {
    pub file_id: String,
    pub metadata: Value,
}

#[derive(Clone, Debug)]
pub struct FileContentResponse {
    pub content: Vec<u8>,
    pub mime_type: String,
    pub metadata: Value,
}

#[derive(Clone, Debug)]
pub struct BatchTaskListResponse {
    pub tasks: Vec<BatchTaskInfo>,
    pub metadata: Value,
}

#[derive(Clone, Debug)]
pub struct BatchTaskInfo {
    pub id: String,
    pub status: String,
    pub created_at: String,
    pub metadata: Value,
}

#[derive(Clone, Debug)]
pub struct BatchTaskRequest {
    pub tasks: Vec<Value>,
    pub metadata: Option<Value>,
}

#[derive(Clone, Debug)]
pub struct BatchTaskResponse {
    pub task_id: String,
    pub status: String,
    pub metadata: Value,
}

#[derive(Clone, Debug)]
pub struct KnowledgeRetrieveRequest {
    pub kb_id: String,
    pub query: String,
    pub top_k: Option<u32>,
    pub metadata: Option<Value>,
}

#[derive(Clone, Debug)]
pub struct KnowledgeRetrieveResponse {
    pub results: Vec<KnowledgeResult>,
    pub metadata: Value,
}

#[derive(Clone, Debug)]
pub struct KnowledgeResult {
    pub content: String,
    pub score: f32,
    pub source: Option<String>,
    pub metadata: Value,
}

#[derive(Clone, Debug)]
pub struct KnowledgeBaseListResponse {
    pub knowledge_bases: Vec<KnowledgeBaseInfo>,
    pub metadata: Value,
}

#[derive(Clone, Debug)]
pub struct KnowledgeBaseInfo {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub metadata: Value,
}

#[derive(Clone, Debug)]
pub struct KnowledgeBaseCreateRequest {
    pub name: String,
    pub description: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Clone, Debug)]
pub struct KnowledgeBaseResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub metadata: Value,
}

#[derive(Clone, Debug)]
pub struct KnowledgeBaseUpdateRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Clone, Debug)]
pub struct KnowledgeBaseUsageResponse {
    pub usage: Value,
    pub metadata: Value,
}

#[derive(Clone, Debug)]
pub struct DocumentListResponse {
    pub documents: Vec<DocumentInfo>,
    pub metadata: Value,
}

#[derive(Clone, Debug)]
pub struct DocumentInfo {
    pub id: String,
    pub name: String,
    pub status: String,
    pub created_at: String,
    pub metadata: Value,
}

#[derive(Clone, Debug)]
pub struct DocumentUploadRequest {
    pub name: String,
    pub content: Vec<u8>,
    pub mime_type: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Clone, Debug)]
pub struct DocumentUrlRequest {
    pub url: String,
    pub name: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Clone, Debug)]
pub struct DocumentImageParseRequest {
    pub image_url: String,
    pub name: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Clone, Debug)]
pub struct DocumentResponse {
    pub id: String,
    pub name: String,
    pub status: String,
    pub metadata: Value,
}

#[derive(Clone, Debug)]
pub struct AssistantChatRequest {
    pub assistant_id: String,
    pub message: String,
    pub session_id: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Clone, Debug)]
pub struct AssistantChatResponse {
    pub response: String,
    pub session_id: String,
    pub metadata: Value,
}

