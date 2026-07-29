use crate::client;

pub fn close(stream_id: impl std::fmt::Display) -> StreamCloseEvent {
    StreamCloseEvent {
        stream_id: stream_id.to_string(),
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StreamCloseEvent {
    pub stream_id: String,
}

impl StreamCloseEvent {
    pub fn into_signal(self) -> client::Signal {
        client::stream::StreamEvent::Close(self).into()
    }
}
