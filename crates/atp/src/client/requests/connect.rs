use serde_valid::Validate;

use crate::types;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Validate)]
pub struct ConnectRequest {
    pub name: String,
    pub description: String,

    #[serde(default)]
    #[validate]
    pub skills: Vec<types::Skill>,
}

impl Default for ConnectRequest {
    fn default() -> Self {
        Self {
            name: env!("CARGO_PKG_NAME").to_string(),
            description: env!("CARGO_PKG_DESCRIPTION").to_string(),
            skills: vec![],
        }
    }
}
