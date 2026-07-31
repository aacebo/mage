#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Notification<T = serde_json::Value> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<uuid::Uuid>,
    pub name: String,

    #[serde(flatten)]
    pub body: T,
}
