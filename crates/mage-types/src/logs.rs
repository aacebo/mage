use std::collections::BTreeMap;

use crate::actors;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Log {
    pub id: uuid::Uuid,
    pub trace_id: uuid::Uuid,
    pub tenant_id: uuid::Uuid,
    pub task_id: Option<uuid::Uuid>,
    pub level: Level,
    pub source: String,
    pub message: String,
    pub fields: BTreeMap<String, serde_json::Value>,
    pub created_by: actors::ActorPartial,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl Level {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Trace => "trace",
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

impl std::fmt::Display for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
