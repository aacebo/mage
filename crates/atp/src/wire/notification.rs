#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Notification<T = serde_json::Value> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<uuid::Uuid>,
    pub name: String,
    pub body: T,
}

impl Notification {
    pub fn try_cast_into<T>(self) -> crate::Result<Notification<T>>
    where
        T: for<'a> serde::Deserialize<'a>,
    {
        Ok(Notification {
            task_id: self.task_id,
            name: self.name,
            body: serde_json::from_value(self.body)?,
        })
    }
}

impl<T> Notification<T> {
    pub fn cast_with<V>(self, body: V) -> Notification<V> {
        Notification {
            task_id: self.task_id,
            name: self.name,
            body,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Error, client, wire};

    #[test]
    fn serde() -> Result<(), Error> {
        let frame: wire::Notification = serde_json::from_value(serde_json::json!({
            "name": "stream.activity",
            "body": {
                "stream_id": "test-123",
                "sequence": 3,
                "phase": "thinking",
                "message": "Thinking..."
            }
        }))?;

        let value: wire::Notification<client::ClientEvent> = frame.try_cast_into()?;
        let event = value.body.clone().try_into_stream()?;
        debug_assert_eq!(event.name(), "stream.activity");
        let json = serde_json::to_string(&value)?;
        debug_assert_eq!(
            json,
            r#"{"name":"stream.activity","body":{"stream_id":"test-123","sequence":3,"phase":"thinking","message":"Thinking..."}}"#,
            "{json}"
        );

        Ok(())
    }
}
