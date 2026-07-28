pub fn error(code: impl std::fmt::Display, message: impl std::fmt::Display) -> Error {
    Error {
        code: code.to_string(),
        message: message.to_string(),
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Signal {
    Ack,
    Error(Error),
    Message(Message),
}

impl Signal {
    pub fn as_error(&self) -> Option<&Error> {
        match self {
            Self::Error(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_message(&self) -> Option<&Message> {
        match self {
            Self::Message(v) => Some(v),
            _ => None,
        }
    }
}

impl From<Error> for Signal {
    fn from(value: Error) -> Self {
        Self::Error(value)
    }
}

impl From<Message> for Signal {
    fn from(value: Message) -> Self {
        Self::Message(value)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Error {
    pub code: String,
    pub message: String,
}

impl Error {
    pub fn into_signal(self) -> Signal {
        self.into()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Message {
    pub id: uuid::Uuid,
    pub chat: types::chats::ChatPartial,
    pub content: types::data::Contents,
    pub metadata: types::data::Metadata,
    pub created_by: types::actors::ActorPartial,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Message {
    pub fn into_signal(self) -> Signal {
        self.into()
    }
}

impl From<types::chats::Message> for Message {
    fn from(value: types::chats::Message) -> Self {
        Self {
            id: value.id,
            chat: value.chat,
            content: value.content,
            metadata: value.metadata,
            created_by: value.created_by,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}
