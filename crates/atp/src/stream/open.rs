pub fn open(stream_id: impl std::fmt::Display) -> StreamOpenEvent {
    StreamOpenEvent {
        stream_id: stream_id.to_string(),
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StreamOpenEvent {
    pub stream_id: String,
}
