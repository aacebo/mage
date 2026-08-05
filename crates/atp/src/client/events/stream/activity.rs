#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StreamActivityEvent {
    pub stream_id: String,
    pub sequence: usize,
    pub phase: StreamPhase,
    pub message: String,
}

#[derive(Debug, Copy, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamPhase {
    Thinking,
    Planning,
    Working,
    Waiting,
}
