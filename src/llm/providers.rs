use super::{LlmProvider, LlmRequest, LlmResponse, LlmUsage};
use anyhow::Result;
use async_trait::async_trait;

fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

async fn read_json(resp: reqwest::Response, provider: &str) -> Result<serde_json::Value> {
    let status = resp.status();
    let json: serde_json::Value = resp.json().await?;
    if !status.is_success() {
        let message = json["error"]["message"]
            .as_str()
            .or_else(|| json["error"].as_str())
            .unwrap_or("request failed");
        anyhow::bail!("{provider} request failed ({status}): {message}");
    }
    Ok(json)
}

/// Ollama provider (local, OpenAI-compatible API)
pub struct OllamaProvider {
    base_url: String,
    client: reqwest::Client,
}

impl OllamaProvider {
    pub fn new(base_url: Option<&str>) -> Self {
        Self {
            base_url: base_url.unwrap_or("http://localhost:11434").to_string(),
            client: http_client(),
        }
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    async fn chat(&self, request: &LlmRequest) -> Result<LlmResponse> {
        let url = format!("{}/v1/chat/completions", self.base_url);
        let body = serde_json::json!({
            "model": request.model,
            "messages": request.messages,
            "max_tokens": request.max_tokens,
            "temperature": request.temperature,
        });

        let resp = self.client.post(&url).json(&body).send().await?;
        let json = read_json(resp, "ollama").await?;

        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let usage = json.get("usage").map(|usage| LlmUsage {
            prompt_tokens: usage["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            completion_tokens: usage["completion_tokens"].as_u64().unwrap_or(0) as u32,
        });

        Ok(LlmResponse {
            content,
            model: request.model.clone(),
            usage,
        })
    }
}

/// Anthropic provider (Claude API)
pub struct AnthropicProvider {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
}

impl AnthropicProvider {
    pub fn new(api_key: &str, base_url: Option<&str>) -> Self {
        Self {
            api_key: api_key.to_string(),
            base_url: base_url.unwrap_or("https://api.anthropic.com").to_string(),
            client: http_client(),
        }
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn chat(&self, request: &LlmRequest) -> Result<LlmResponse> {
        let url = format!("{}/v1/messages", self.base_url);

        // Convert messages to Anthropic format (separate system from user messages)
        let system = request
            .messages
            .iter()
            .find(|m| m.role == "system")
            .map(|m| m.content.clone());

        let messages: Vec<serde_json::Value> = request
            .messages
            .iter()
            .filter(|m| m.role != "system")
            .map(|m| serde_json::json!({"role": m.role, "content": m.content}))
            .collect();

        let mut body = serde_json::json!({
            "model": request.model,
            "messages": messages,
            "max_tokens": request.max_tokens,
            "temperature": request.temperature,
        });

        if let Some(sys) = system {
            body["system"] = serde_json::Value::String(sys);
        }

        let resp = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        let json = read_json(resp, "anthropic").await?;
        let content = json["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(LlmResponse {
            content,
            model: request.model.clone(),
            usage: Some(LlmUsage {
                prompt_tokens: json["usage"]["input_tokens"].as_u64().unwrap_or(0) as u32,
                completion_tokens: json["usage"]["output_tokens"].as_u64().unwrap_or(0) as u32,
            }),
        })
    }
}

/// OpenAI provider (GPT-4, etc.)
pub struct OpenAiProvider {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
}

impl OpenAiProvider {
    pub fn new(api_key: &str, base_url: Option<&str>) -> Self {
        Self {
            api_key: api_key.to_string(),
            base_url: base_url.unwrap_or("https://api.openai.com").to_string(),
            client: http_client(),
        }
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn chat(&self, request: &LlmRequest) -> Result<LlmResponse> {
        let url = format!("{}/v1/chat/completions", self.base_url);
        let body = serde_json::json!({
            "model": request.model,
            "messages": request.messages,
            "max_tokens": request.max_tokens,
            "temperature": request.temperature,
        });

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await?;

        let json = read_json(resp, "openai").await?;
        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(LlmResponse {
            content,
            model: request.model.clone(),
            usage: Some(LlmUsage {
                prompt_tokens: json["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32,
                completion_tokens: json["usage"]["completion_tokens"].as_u64().unwrap_or(0) as u32,
            }),
        })
    }
}
