use serde_valid::Validate;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Actor {
    pub id: uuid::Uuid,
    pub role: Role,
    pub name: String,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    User,
    Agent,
}

impl Role {
    pub fn is_user(self) -> bool {
        matches!(self, Self::User)
    }

    pub fn is_agent(self) -> bool {
        matches!(self, Self::Agent)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Agent => "agent",
        }
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
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
