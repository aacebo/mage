#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StreamStatusEvent {
    pub stream_id: String,
    pub sequence: usize,
    pub code: StatusCode,
    pub message: String,
}

#[derive(Debug, Copy, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusCode {
    Thinking,
    Planning,
    Working,
    Waiting,
}
