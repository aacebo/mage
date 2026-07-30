use std::collections::BTreeMap;

use crate::types;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MessageEvent {
    pub id: uuid::Uuid,
    pub chat: types::Chat,
    pub content: Vec<types::Content>,
    pub metadata: BTreeMap<String, serde_json::Value>,
    pub created_by: types::Actor,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
