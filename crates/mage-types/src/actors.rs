use serde_valid::Validate;

use crate::data;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Validate)]
pub struct Actor {
    pub id: uuid::Uuid,
    pub tenant_id: uuid::Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    pub role: Role,
    pub name: String,
    #[serde(flatten)]
    pub agent: Option<Agent>,
    pub metadata: data::Metadata,
    #[serde(skip)]
    pub embedding: Option<Vec<f32>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Validate)]
pub struct ActorPartial {
    pub id: uuid::Uuid,
    pub role: Role,
    pub name: String,
    #[validate]
    #[serde(flatten)]
    pub agent: Option<Agent>,
}

impl From<Actor> for ActorPartial {
    fn from(value: Actor) -> Self {
        Self {
            id: value.id,
            role: value.role,
            name: value.name,
            agent: value.agent,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Validate)]
pub struct Agent {
    pub status: AgentStatus,
    pub description: String,
    #[serde(skip)]
    pub secret: String,
    pub instances: u32,
    #[validate]
    pub skills: Vec<Skill>,
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

#[derive(Debug, Copy, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Online,
    Offline,
}

impl AgentStatus {
    pub fn is_online(self) -> bool {
        matches!(self, Self::Online)
    }

    pub fn is_offline(self) -> bool {
        matches!(self, Self::Offline)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Online => "online",
            Self::Offline => "offline",
        }
    }
}

impl std::fmt::Display for AgentStatus {
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
