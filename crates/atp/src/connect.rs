use serde_valid::Validate;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Validate)]
pub struct Connect {
    pub name: String,
    pub description: String,

    #[serde(default)]
    #[validate]
    pub skills: Vec<Skill>,
}

impl Default for Connect {
    fn default() -> Self {
        Self {
            name: env!("CARGO_PKG_NAME").to_string(),
            description: env!("CARGO_PKG_DESCRIPTION").to_string(),
            skills: vec![],
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Validate)]
pub struct Skill {
    #[validate(pattern = r"^([a-z0-9_]+)$")]
    pub name: String,

    pub display_name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}
