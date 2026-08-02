#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Request<T = serde_json::Value> {
    pub id: uuid::Uuid,
    pub method: String,
    pub params: T,
}

impl Request {
    pub fn try_cast_into<T>(self) -> Result<Request<T>, crate::Error>
    where
        T: for<'a> serde::Deserialize<'a>,
    {
        Ok(Request {
            id: self.id,
            method: self.method,
            params: serde_json::from_value(self.params)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{Error, client, wire};

    #[test]
    fn round_trip() -> Result<(), Error> {
        let frame: wire::Request = serde_json::from_value(serde_json::json!({
            "id": "019fb92c-e616-716f-9768-16c4753fe9d8",
            "method": "connect",
            "params": {
                "name": "test",
                "description": "a test agent...",
                "secret": "abcdefg",
                "skills": [
                    {
                        "name": "echo",
                        "display_name": "Echo",
                        "description": "I can echo back what you said to me"
                    }
                ]
            }
        }))?;

        let value: wire::Request<client::ClientParams> = frame.try_cast_into()?;
        debug_assert_eq!(value.params.try_connect()?.name, "test");
        let json = serde_json::to_string(&value)?;
        debug_assert_eq!(
            json,
            r#"{"id":"019fb92c-e616-716f-9768-16c4753fe9d8","method":"connect","params":{"name":"test","description":"a test agent...","secret":"abcdefg","skills":[{"name":"echo","display_name":"Echo","description":"I can echo back what you said to me"}]}}"#,
            "{json}"
        );

        Ok(())
    }
}
