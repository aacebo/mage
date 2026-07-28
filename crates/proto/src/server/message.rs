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
    pub fn into_signal(self) -> super::Signal {
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
