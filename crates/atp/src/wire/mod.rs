mod notification;
mod request;
mod response;

pub use notification::*;
pub use request::*;
pub use response::*;

use crate::error;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum Frame<T = serde_json::Value> {
    Notification(Notification<T>),
    Request(Request<T>),
    Response(Response<T>),
}

impl<T> Frame<T> {
    pub fn is_request(&self) -> bool {
        matches!(self, Self::Request(_))
    }

    pub fn is_response(&self) -> bool {
        matches!(self, Self::Response(_))
    }

    pub fn is_notification(&self) -> bool {
        matches!(self, Self::Notification(_))
    }

    pub fn try_request(&self) -> Result<&Request<T>, crate::Error> {
        match self {
            Self::Request(v) => Ok(v),
            _ => Err(error::protocol("expected request frame")),
        }
    }

    pub fn try_response(&self) -> Result<&Response<T>, crate::Error> {
        match self {
            Self::Response(v) => Ok(v),
            _ => Err(error::protocol("expected response frame")),
        }
    }

    pub fn try_notification(&self) -> Result<&Notification<T>, crate::Error> {
        match self {
            Self::Notification(v) => Ok(v),
            _ => Err(error::protocol("expected notification frame")),
        }
    }

    pub fn result(&self) -> Result<Option<&T>, &Error> {
        match self {
            Self::Request(v) => Ok(Some(&v.params)),
            Self::Response(v) => v.result(),
            Self::Notification(v) => Ok(Some(&v.body)),
        }
    }
}

impl Frame {
    pub fn try_cast_into<T>(self) -> Result<Frame<T>, crate::Error>
    where
        T: for<'a> serde::Deserialize<'a>,
    {
        match self {
            Self::Request(v) => Ok(v.try_cast_into()?.into()),
            Self::Response(v) => Ok(v.try_cast_into()?.into()),
            Self::Notification(v) => Ok(v.try_cast_into()?.into()),
        }
    }
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
    use crate::{Error, client, wire};

    #[test]
    fn notification() -> Result<(), Error> {
        let frame: wire::Frame = serde_json::from_value(serde_json::json!({
            "name": "stream.status",
            "body": {
                "stream_id": "test-123",
                "sequence": 3,
                "code": "thinking",
                "message": "Thinking..."
            }
        }))?;

        let value: wire::Notification<client::ClientEvent> = frame.try_notification()?.clone().try_cast_into()?;
        let event = value.body.try_stream()?;
        debug_assert_eq!(event.name(), "stream.status");
        let json = serde_json::to_string(&value)?;
        debug_assert_eq!(
            json,
            r#"{"name":"stream.status","body":{"stream_id":"test-123","sequence":3,"code":"thinking","message":"Thinking..."}}"#,
            "{json}"
        );

        Ok(())
    }

    #[test]
    fn request() -> Result<(), Error> {
        let frame: wire::Frame = serde_json::from_value(serde_json::json!({
            "id": "019fb92c-e616-716f-9768-16c4753fe9d8",
            "method": "connect",
            "params": {
                "id": "019fb92c-e616-716f-9768-16c4753fe9d9",
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

        let value: wire::Request<client::ClientParams> = frame.try_request()?.clone().try_cast_into()?;
        debug_assert_eq!(
            value.params.try_connect()?.id,
            "019fb92c-e616-716f-9768-16c4753fe9d9".parse::<uuid::Uuid>().unwrap()
        );
        debug_assert_eq!(value.params.try_connect()?.name, "test");
        let json = serde_json::to_string(&value)?;
        debug_assert_eq!(
            json,
            r#"{"id":"019fb92c-e616-716f-9768-16c4753fe9d8","method":"connect","params":{"id":"019fb92c-e616-716f-9768-16c4753fe9d9","name":"test","description":"a test agent...","secret":"abcdefg","skills":[{"name":"echo","display_name":"Echo","description":"I can echo back what you said to me"}]}}"#,
            "{json}"
        );

        Ok(())
    }

    mod response {
        use super::*;

        #[test]
        fn error() -> Result<(), Error> {
            let frame: wire::Frame = serde_json::from_value(serde_json::json!({
                "id": "019fb92c-e616-716f-9768-16c4753fe9d8",
                "error": {
                    "code": -200,
                    "message": "??"
                }
            }))?;

            let error = frame.try_response()?.result().unwrap_err();
            debug_assert_eq!(error.code, -200, "{error:#?}");
            debug_assert_eq!(error.message, "??", "{error:#?}");
            let json = serde_json::to_string(&frame)?;
            debug_assert_eq!(
                json, r#"{"id":"019fb92c-e616-716f-9768-16c4753fe9d8","error":{"code":-200,"message":"??"}}"#,
                "{json}"
            );

            Ok(())
        }

        #[test]
        fn result() -> Result<(), Error> {
            let frame: wire::Frame = serde_json::from_value(serde_json::json!({
                "id": "019fb92c-e616-716f-9768-16c4753fe9d8",
                "result": 200
            }))?;

            let res = frame.try_response()?.result().unwrap();
            debug_assert_eq!(res.cloned(), Some(serde_json::to_value(200)?), "{frame:#?}");
            let json = serde_json::to_string(&frame)?;
            debug_assert_eq!(
                json, r#"{"id":"019fb92c-e616-716f-9768-16c4753fe9d8","result":200}"#,
                "{json}"
            );

            Ok(())
        }
    }
}
