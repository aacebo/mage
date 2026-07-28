pub mod error;
pub mod message;

pub fn error(code: impl std::fmt::Display, message: impl std::fmt::Display) -> error::Error {
    error::Error {
        code: code.to_string(),
        message: message.to_string(),
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Signal {
    Ack,
    Error(error::Error),
    Message(message::Message),
}

impl Signal {
    pub fn as_error(&self) -> Option<&error::Error> {
        match self {
            Self::Error(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_message(&self) -> Option<&message::Message> {
        match self {
            Self::Message(v) => Some(v),
            _ => None,
        }
    }
}

impl From<error::Error> for Signal {
    fn from(value: error::Error) -> Self {
        Self::Error(value)
    }
}

impl From<message::Message> for Signal {
    fn from(value: message::Message) -> Self {
        Self::Message(value)
    }
}
