pub mod client;
pub mod server;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProtocolMessage<T> {
    pub id: uuid::Uuid,
    pub trace_id: uuid::Uuid,
    pub reply_to_id: Option<uuid::Uuid>,
    pub body: T,
    pub sent_at: chrono::DateTime<chrono::Utc>,
}

impl<T> ProtocolMessage<T> {
    pub fn new(trace_id: uuid::Uuid, body: impl Into<T>) -> Self {
        Self {
            id: uuid::Uuid::now_v7(),
            trace_id,
            reply_to_id: None,
            body: body.into(),
            sent_at: chrono::Utc::now(),
        }
    }

    pub fn reply<V>(&self, body: V) -> ProtocolMessage<V> {
        ProtocolMessage {
            id: uuid::Uuid::now_v7(),
            trace_id: self.trace_id,
            reply_to_id: Some(self.id),
            body,
            sent_at: chrono::Utc::now(),
        }
    }
}
