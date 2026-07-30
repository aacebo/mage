mod message;

pub use message::MessageEvent;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "name", content = "body", rename_all = "snake_case")]
pub enum ServerEvent {
    Message(MessageEvent),
}

impl ServerEvent {
    pub fn as_message(&self) -> Option<&MessageEvent> {
        match self {
            Self::Message(v) => Some(v),
            #[allow(unused)]
            _ => None,
        }
    }
}

impl From<MessageEvent> for ServerEvent {
    fn from(value: MessageEvent) -> Self {
        Self::Message(value)
    }
}
