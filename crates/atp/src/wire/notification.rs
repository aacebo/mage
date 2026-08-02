#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Notification<T = serde_json::Value> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<uuid::Uuid>,

    #[serde(flatten)]
    pub body: T,
}

impl Notification {
    pub fn try_cast_into<T>(self) -> Result<Notification<T>, crate::Error>
    where
        T: for<'a> serde::Deserialize<'a>,
    {
        Ok(Notification {
            task_id: self.task_id,
            body: serde_json::from_value(self.body)?,
        })
    }
}
