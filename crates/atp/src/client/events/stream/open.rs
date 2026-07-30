#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StreamOpenEvent {
    pub stream_id: String,
    pub sequence: usize,
}
