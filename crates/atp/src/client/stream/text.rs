use crate::client;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StreamTextEvent {
    pub stream_id: String,
    pub index: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item: Option<usize>,
    pub text: String,
    pub status: Option<String>,
}

impl StreamTextEvent {
    pub fn into_signal(self) -> client::Signal {
        client::stream::StreamEvent::Text(self).into()
    }
}
