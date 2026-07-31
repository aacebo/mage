mod notification;
mod request;
mod response;

pub use notification::*;
pub use request::*;
pub use response::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum Frame<T = serde_json::Value> {
    Notification(Notification<T>),
    Response(Response<T>),
    Request(Request<T>),
}

impl<T> From<Request<T>> for Frame<T> {
    fn from(value: Request<T>) -> Self {
        Self::Request(value)
    }
}

impl<T> From<Response<T>> for Frame<T> {
    fn from(value: Response<T>) -> Self {
        Self::Response(value)
    }
}

impl<T> From<Notification<T>> for Frame<T> {
    fn from(value: Notification<T>) -> Self {
        Self::Notification(value)
    }
}

#[cfg(test)]
mod tests {
    use crate::{Error, wire};

    mod serde {
        use super::*;

        #[test]
        fn request() -> Result<(), Error> {
            let frame: wire::Frame = serde_json::from_str(
                r#"{
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
            }"#,
            )?;

            let json = serde_json::to_string(&frame)?;

            debug_assert_eq!(
                json,
                r#"{"method":"connect","params":{"description":"a test agent...","name":"test","secret":"abcdefg","skills":[{"description":"I can echo back what you said to me","display_name":"Echo","name":"echo"}]}}"#,
                "{json}"
            );

            Ok(())
        }

        #[test]
        fn notification() -> Result<(), Error> {
            let frame: wire::Frame = serde_json::from_str(
                r#"{
                "name": "stream.status",
                "body": {
                    "stream_id": "1",
                    "sequence": 3,
                    "code": "thinking",
                    "message": "thinking..."
                }
            }"#,
            )?;

            let json = serde_json::to_string(&frame)?;

            debug_assert_eq!(
                json,
                r#"{"body":{"code":"thinking","message":"thinking...","sequence":3,"stream_id":"1"},"name":"stream.status"}"#,
                "{json}"
            );

            Ok(())
        }
    }
}
