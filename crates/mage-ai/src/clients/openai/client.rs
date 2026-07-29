use std::time::Duration;

use mage_error::Result;
use reqwest::blocking::Client;

use super::types::{ChatResponse, EmbeddingsResponse, Message};
use crate::models::{ModelId, Provider};

/// An OpenAI-compatible client. The same schema is spoken by Azure OpenAI, Ollama, vLLM,
/// LM Studio, OpenRouter and Together, so `base_url` is all that differs between them.
pub struct OpenAI {
    client: Client,
    base_url: String,
    api_key: Option<String>,
    model: ModelId,
}

impl OpenAI {
    pub fn new(model: ModelId, base_url: Option<String>, api_key: Option<String>) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(120))
                .build()
                .unwrap_or_else(|_| Client::new()),
            base_url: base_url.unwrap_or_else(|| Provider::OpenAI.base_url().to_string()),
            api_key: api_key.or_else(|| std::env::var(Provider::OpenAI.env()).ok()),
            model,
        }
    }

    /// A chat completion constrained to a JSON schema, so the model answers in a parseable shape
    /// rather than prose.
    pub fn json<T: serde::de::DeserializeOwned>(&self, prompt: &str, input: &str, schema: serde_json::Value) -> Result<T> {
        let body = serde_json::json!({
            "model": self.model.to_string(),
            "messages": [Message::system(prompt), Message::user(input)],
            "response_format": {
                "type": "json_schema",
                "json_schema": { "name": "result", "strict": true, "schema": schema },
            },
        });

        let response: ChatResponse = self.post("chat/completions", &body)?;
        let content = response
            .choices
            .into_iter()
            .next()
            .map(|choice| choice.message.content)
            .ok_or_else(|| mage_error::ai("no completion returned"))?;

        Ok(serde_json::from_str(&content)?)
    }

    /// A plain chat completion. Generation returns prose, so unlike `json` it is not constrained
    /// to a schema -- wrapping a summary in a JSON envelope only to unwrap it buys nothing.
    pub fn complete(&self, prompt: &str, input: &str, max_len: Option<usize>) -> Result<String> {
        let mut body = serde_json::json!({
            "model": self.model.to_string(),
            "messages": [Message::system(prompt), Message::user(input)],
        });

        if let Some(max_len) = max_len {
            body["max_completion_tokens"] = serde_json::json!(max_len);
        }

        let response: ChatResponse = self.post("chat/completions", &body)?;

        response
            .choices
            .into_iter()
            .next()
            .map(|choice| choice.message.content.trim().to_string())
            .ok_or_else(|| mage_error::ai("no completion returned"))
    }

    pub fn embeddings(&self, text: &[&str]) -> Result<Vec<Vec<f32>>> {
        let body = serde_json::json!({
            "model": self.model.to_string(),
            "input": text,
        });

        let response: EmbeddingsResponse = self.post("embeddings", &body)?;
        let mut data = response.data;

        if data.len() != text.len() {
            return Err(mage_error::ai(format!(
                "expected {} embeddings, got {}",
                text.len(),
                data.len()
            )));
        }

        data.sort_by_key(|entry| entry.index);
        Ok(data.into_iter().map(|entry| entry.embedding).collect())
    }

    fn post<T: serde::de::DeserializeOwned>(&self, path: &str, body: &serde_json::Value) -> Result<T> {
        let url = format!("{}/{path}", self.base_url.trim_end_matches('/'));
        let mut request = self.client.post(&url).json(body);

        if let Some(key) = &self.api_key {
            request = request.bearer_auth(key);
        }

        let response = request.send()?;
        let status = response.status();

        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            return Err(mage_error::http(format!("{url} returned {status}")));
        }

        if !status.is_success() {
            let body = response.text().unwrap_or_default();
            return Err(mage_error::http(format!("{url} returned {status}: {body}")));
        }

        Ok(response.json()?)
    }
}
