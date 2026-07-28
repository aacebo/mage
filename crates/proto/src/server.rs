use serde_valid::Validate;

pub fn error(code: impl std::fmt::Display, message: impl std::fmt::Display) -> Error {
    Error {
        code: code.to_string(),
        message: message.to_string(),
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Validate)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Signal {
    Error(Error),
    #[validate]
    Message(types::chats::Message),
}

impl From<Error> for Signal {
    fn from(value: Error) -> Self {
        Self::Error(value)
    }
}

impl From<types::chats::Message> for Signal {
    fn from(value: types::chats::Message) -> Self {
        Self::Message(value)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Error {
    pub code: String,
    pub message: String,
}
