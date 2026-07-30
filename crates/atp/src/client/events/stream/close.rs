#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StreamCloseEvent {
    pub stream_id: String,
    pub sequence: usize,
    pub reason: CloseReason,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Copy, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CloseReason {
    Completed,
    Cancelled,
    Failed,
}
