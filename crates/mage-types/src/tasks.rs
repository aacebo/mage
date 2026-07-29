use crate::actors;

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct Task {
    pub id: uuid::Uuid,
    pub trace_id: uuid::Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<uuid::Uuid>,
    pub name: String,
    pub status: TaskStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<serde_json::Value>,
    pub attempts: i32,
    pub max_attempts: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Task {
    pub fn new(name: impl Into<String>) -> Self {
        let now = chrono::Utc::now();

        Self {
            id: uuid::Uuid::now_v7(),
            name: name.into(),
            max_attempts: 3,
            created_at: now,
            updated_at: now,
            ..Default::default()
        }
    }

    pub fn start(mut self) -> Self {
        assert!(self.status != TaskStatus::Running);
        let now = chrono::Utc::now();

        self.status = TaskStatus::Running;
        self.attempts += 1;
        self.started_at = Some(now);
        self.error = None;
        self.ended_at = None;
        self.updated_at = now;
        self
    }

    pub fn success(mut self) -> Self {
        assert!(self.status == TaskStatus::Running);
        let now = chrono::Utc::now();

        self.status = TaskStatus::Success;
        self.ended_at = Some(now);
        self.updated_at = now;
        self
    }

    pub fn fail(mut self, error: impl Into<serde_json::Value>) -> Self {
        assert!(self.status == TaskStatus::Running);
        let now = chrono::Utc::now();

        self.error = Some(error.into());
        self.status = TaskStatus::Failure;
        self.ended_at = Some(now);
        self.updated_at = now;
        self
    }

    pub fn cancel(mut self) -> Self {
        assert!(self.status == TaskStatus::Running);
        let now = chrono::Utc::now();

        self.status = TaskStatus::Cancelled;
        self.ended_at = Some(now);
        self.updated_at = now;
        self
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    #[default]
    Queued,
    Running,
    Failure,
    Cancelled,
    Success,
}

impl TaskStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Failure => "failure",
            Self::Cancelled => "cancelled",
            Self::Success => "success",
        }
    }
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TaskEvent {
    pub id: uuid::Uuid,
    pub tenant_id: uuid::Uuid,
    pub task_id: uuid::Uuid,
    pub sequence: u64,
    #[serde(flatten)]
    pub data: TaskEventData,
    pub created_by: actors::ActorPartial,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum TaskEventData {
    Started,
    Status,
    Progress,
    TextDelta,
    Message,
    Artifact,
    ToolCall,
    ToolResult,
    Warning,
    Error,
    Completed,
    Cancelled,
    Custom(String),
}
