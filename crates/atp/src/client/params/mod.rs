use serde_valid::Validate;

mod connect;
mod message;

pub use connect::*;
pub use message::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Validate)]
#[serde(untagged)]
pub enum ClientParams {
    #[validate]
    Connect(ConnectParams),

    #[validate]
    Message(MessageParams),
}

impl ClientParams {
    pub fn try_connect(&self) -> Result<&ConnectParams, crate::Error> {
        match self {
            Self::Connect(v) => Ok(v),
            _ => Err(crate::error::protocol("expected connect request")),
        }
    }

    pub fn try_message(&self) -> Result<&MessageParams, crate::Error> {
        match self {
            Self::Message(v) => Ok(v),
            _ => Err(crate::error::protocol("expected message request")),
        }
    }
}

impl From<ConnectParams> for ClientParams {
    fn from(value: ConnectParams) -> Self {
        Self::Connect(value)
    }
}

impl From<MessageParams> for ClientParams {
    fn from(value: MessageParams) -> Self {
        Self::Message(value)
    }
}
