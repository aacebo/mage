use serde_valid::Validate;

mod connect;
mod message;

pub use connect::*;
pub use message::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Validate)]
#[serde(tag = "method", content = "params", rename_all = "snake_case")]
pub enum ClientRequest {
    #[validate]
    Connect(ConnectRequest),

    #[validate]
    Message(MessageRequest),
}

impl ClientRequest {
    pub fn as_connect(&self) -> Option<&ConnectRequest> {
        match self {
            Self::Connect(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_message(&self) -> Option<&MessageRequest> {
        match self {
            Self::Message(v) => Some(v),
            _ => None,
        }
    }
}

impl From<ConnectRequest> for ClientRequest {
    fn from(value: ConnectRequest) -> Self {
        Self::Connect(value)
    }
}

impl From<MessageRequest> for ClientRequest {
    fn from(value: MessageRequest) -> Self {
        Self::Message(value)
    }
}
