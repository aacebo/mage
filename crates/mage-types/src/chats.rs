use serde_valid::Validate;

use crate::{actors, data};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Chat {
    pub id: uuid::Uuid,
    pub tenant_id: uuid::Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub created_by: actors::ActorPartial,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closed_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChatPartial {
    pub id: uuid::Uuid,
    pub tenant_id: uuid::Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl From<Chat> for ChatPartial {
    fn from(value: Chat) -> Self {
        Self {
            id: value.id,
            tenant_id: value.tenant_id,
            name: value.name,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Validate)]
pub struct Message {
    pub id: uuid::Uuid,
    pub chat: ChatPartial,
    #[validate]
    pub content: data::Contents,
    pub metadata: data::Metadata,
    #[serde(skip)]
    pub embedding: Option<Vec<f32>>,
    #[validate]
    pub created_by: actors::ActorPartial,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Validate)]
pub struct InboundMessage {
    pub tenant_id: uuid::Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chat_id: Option<uuid::Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[validate]
    pub content: data::Contents,
    pub metadata: data::Metadata,
    #[validate]
    pub sent_by: actors::ActorPartial,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChatMembers {
    pub chat_id: uuid::Uuid,
    pub actor_ids: Vec<uuid::Uuid>,
}
