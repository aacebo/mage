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
    pub fn method(&self) -> &'static str {
        match self {
            Self::Connect(_) => "connect",
            Self::Message(_) => "message",
        }
    }

    pub fn try_into_connect(self) -> Result<ConnectParams, Box<dyn std::error::Error>> {
        match self {
            Self::Connect(v) => Ok(v),
            v => Err(crate::error::invalid_request(format!("expected `connect`, received `{}`", v.method())).into()),
        }
    }

    pub fn try_into_message(self) -> Result<MessageParams, Box<dyn std::error::Error>> {
        match self {
            Self::Message(v) => Ok(v),
            v => Err(crate::error::invalid_request(format!("expected `message`, received `{}`", v.method())).into()),
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
