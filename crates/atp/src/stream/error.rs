#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StreamErrorEvent {
    pub stream_id: String,
    pub index: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item: Option<usize>,
    pub code: String,
    pub message: String,
}
