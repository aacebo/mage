#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChatResponse {
    pub choices: Vec<Choice>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Choice {
    pub message: Message,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Message {
    #[serde(default)]
    pub role: String,
    pub content: String,
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EmbeddingsResponse {
    pub data: Vec<Embedding>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Embedding {
    pub index: usize,
    pub embedding: Vec<f32>,
}
