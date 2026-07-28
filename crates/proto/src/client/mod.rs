use serde_valid::Validate;

pub mod connect;
pub mod message;
pub mod stream;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Validate)]
#[serde(tag = "type")]
pub enum Signal {
    #[serde(rename = "connect")]
    #[validate]
    Connect(connect::Connect),

    #[serde(rename = "response.message")]
    #[validate]
    Message(message::MessageResponse),

    #[serde(untagged)]
    #[validate]
    Stream(stream::StreamResponse),
}

impl Signal {
    pub fn as_connect(&self) -> Option<&connect::Connect> {
        match self {
            Self::Connect(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_message_response(&self) -> Option<&message::MessageResponse> {
        match self {
            Self::Message(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_stream_response(&self) -> Option<&stream::StreamResponse> {
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

impl From<message::MessageResponse> for Signal {
    fn from(value: message::MessageResponse) -> Self {
        Self::Message(value)
    }
}

impl From<stream::StreamResponse> for Signal {
    fn from(value: stream::StreamResponse) -> Self {
        Self::Stream(value)
    }
}
