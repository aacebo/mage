pub mod client;
pub mod server;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProtocolMessage<T> {
    pub id: uuid::Uuid,
    pub trace_id: uuid::Uuid,
    pub body: T,
    pub sent_at: chrono::DateTime<chrono::Utc>,
}

impl<T> ProtocolMessage<T> {
    pub fn new(trace_id: uuid::Uuid, body: impl Into<T>) -> Self {
        Self {
            id: uuid::Uuid::now_v7(),
            trace_id,
            body: body.into(),
            sent_at: chrono::Utc::now(),
        }
    }
}
