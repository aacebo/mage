mod message;

pub use message::MessageEvent;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum ServerEvent {
    Message(MessageEvent),
}

impl ServerEvent {
    pub fn try_into_message(self) -> crate::Result<MessageEvent> {
        match self {
            Self::Message(v) => Ok(v),
        }
    }
}

impl From<MessageEvent> for ServerEvent {
    fn from(value: MessageEvent) -> Self {
        Self::Message(value)
    }
}
