use serde_valid::Validate;

use crate::{actors, data};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Validate)]
pub struct Artifact {
    pub id: uuid::Uuid,
    pub name: String,
    #[validate]
    pub content: data::Contents,
    #[serde(skip)]
    pub embedding: Option<Vec<f32>>,
    pub metadata: data::Metadata,
    #[validate]
    pub created_by: actors::Actor,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
