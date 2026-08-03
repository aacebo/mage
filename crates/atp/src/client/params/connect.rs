use serde_valid::Validate;

use crate::types;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Validate)]
pub struct ConnectParams {
    pub id: uuid::Uuid,
    pub name: String,
    pub description: String,
    pub secret: String,

    #[serde(default)]
    #[validate]
    pub skills: Vec<types::Skill>,
}

impl Default for ConnectParams {
    fn default() -> Self {
        Self {
            id: std::env::var("AGENT_ID")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or_default(),
            name: env!("CARGO_PKG_NAME").to_string(),
            description: env!("CARGO_PKG_DESCRIPTION").to_string(),
            secret: std::env::var("AGENT_SECRET").unwrap_or_default(),
            skills: vec![],
        }
    }
}
