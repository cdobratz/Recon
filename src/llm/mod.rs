pub mod providers;
pub mod prompts;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Request to an LLM provider
#[derive(Debug, Clone, Serialize)]
pub struct LlmRequest {
    pub model: String,
    pub messages: Vec<LlmMessage>,
    pub max_tokens: u32,
    pub temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessage {
    pub role: String,
    pub content: String,
}

/// Response from an LLM provider
#[derive(Debug, Clone, Deserialize)]
pub struct LlmResponse {
    pub content: String,
    pub model: String,
    pub usage: Option<LlmUsage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LlmUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}

/// Trait for LLM provider implementations
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Send a chat completion request
    async fn chat(&self, request: &LlmRequest) -> Result<LlmResponse>;

    /// Get the provider name
    fn name(&self) -> &str;

    /// Check if the provider is available/configured
    async fn is_available(&self) -> bool;
}

/// Create an LLM provider from config
pub fn create_provider(
    provider: &str,
    api_key: Option<&str>,
    base_url: Option<&str>,
) -> Result<Box<dyn LlmProvider>> {
    match provider {
        "ollama" => Ok(Box::new(providers::OllamaProvider::new(base_url))),
        "anthropic" => {
            let key = api_key.ok_or_else(|| anyhow::anyhow!("Anthropic requires an API key"))?;
            Ok(Box::new(providers::AnthropicProvider::new(key, base_url)))
        }
        "openai" => {
            let key = api_key.ok_or_else(|| anyhow::anyhow!("OpenAI requires an API key"))?;
            Ok(Box::new(providers::OpenAiProvider::new(key, base_url)))
        }
        _ => anyhow::bail!("Unknown LLM provider: {}", provider),
    }
}
