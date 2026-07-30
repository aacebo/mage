use std::collections::BTreeMap;

use serde_valid::Validate;

use crate::types::Content;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Validate)]
pub struct MessageRequest {
    pub chat_id: uuid::Uuid,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<uuid::Uuid>,

    #[validate(min_items = 1)]
    pub content: Vec<Content>,

    #[serde(default)]
    pub metadata: BTreeMap<String, serde_json::Value>,
}
