pub mod types;
pub mod client;
pub mod echo;
#[cfg(feature = "openai-client")]
pub mod config;
#[cfg(feature = "openai-client")]
pub mod extended;
#[cfg(feature = "openai-client")]
pub mod http;

pub use types::{LlmMessage, LlmRequest, LlmResponse, LlmStreamChunk};
#[cfg(feature = "openai-client")]
pub use types::ApiFormat;
pub use client::{LlmClient, DynLlmClient};
pub use echo::LocalEchoClient;

#[cfg(feature = "openai-client")]
pub use config::ApiEndpointConfig;

#[cfg(feature = "openai-client")]
pub use extended::{
    ExtendedApiClient, DynExtendedApiClient, 
    GenericApiClient,
    UniversalApiClient, ApiBuilder, ApiCallBuilder
};
#[cfg(feature = "openai-client")]
pub use http::GenericHttpClient;
#[cfg(feature = "openai-client")]
pub use http::*;
