use serde_valid::Validate;

pub mod connect;
pub mod message;
pub mod stream;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Validate)]
#[serde(tag = "type")]
pub enum Signal {
    #[serde(rename = "ack")]
    Ack,

    #[serde(rename = "connect")]
    #[validate]
    Connect(connect::Connect),

    #[serde(rename = "message")]
    #[validate]
    Message(message::Message),

    #[serde(untagged)]
    #[validate]
    Stream(stream::StreamEvent),
}

impl Signal {
    pub fn as_connect(&self) -> Option<&connect::Connect> {
        match self {
            Self::Connect(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_message(&self) -> Option<&message::Message> {
        match self {
            Self::Message(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_stream_event(&self) -> Option<&stream::StreamEvent> {
        match self {
            Self::Stream(v) => Some(v),
            _ => None,
        }
    }
}

impl From<connect::Connect> for Signal {
    fn from(value: connect::Connect) -> Self {
        Self::Connect(value)
    }
}

impl From<message::Message> for Signal {
    fn from(value: message::Message) -> Self {
        Self::Message(value)
    }
}

impl From<stream::StreamEvent> for Signal {
    fn from(value: stream::StreamEvent) -> Self {
        Self::Stream(value)
    }
}
