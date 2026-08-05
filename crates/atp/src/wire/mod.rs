mod notification;
mod request;
mod response;

pub use notification::*;
pub use request::*;
pub use response::*;

use crate::{Error, error};

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

    pub fn try_into_request(self) -> crate::Result<Request<T>> {
        match self {
            Self::Request(v) => Ok(v),
            _ => Err(error::invalid_request("expected request")),
        }
    }

    pub fn try_into_response(self) -> crate::Result<Response<T>> {
        match self {
            Self::Response(v) => Ok(v),
            _ => Err(error::invalid_request("expected response")),
        }
    }

    pub fn try_into_notification(self) -> crate::Result<Notification<T>> {
        match self {
            Self::Notification(v) => Ok(v),
            _ => Err(error::invalid_request("expected notification")),
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
    pub fn try_cast_into<T>(self) -> crate::Result<Frame<T>>
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

impl<T> Frame<T> {
    pub fn cast_with<V>(self, data: V) -> Frame<V> {
        match self {
            Self::Request(v) => v.cast_with(data).into(),
            Self::Response(v) => v.cast_with(Some(data)).into(),
            Self::Notification(v) => v.cast_with(data).into(),
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
    fn wrong_frame_kind_and_invalid_cast_are_typed_errors() {
        let notification = wire::Notification {
            task_id: None,
            name: "event".to_string(),
            body: serde_json::Value::Null,
        };
        let error = wire::Frame::from(notification).try_into_request().unwrap_err();
        assert_eq!(error.code, Error::INVALID_REQUEST);

        let request = wire::Request {
            id: uuid::Uuid::now_v7(),
            method: "connect".to_string(),
            params: serde_json::json!(42),
        };
        let error = request.try_cast_into::<uuid::Uuid>().unwrap_err();
        assert_eq!(error.code, Error::INVALID_REQUEST);
    }

    #[test]
    fn notification() -> Result<(), Error> {
        let frame: wire::Frame = serde_json::from_value(serde_json::json!({
            "name": "stream.activity",
            "body": {
                "stream_id": "test-123",
                "sequence": 3,
                "phase": "thinking",
                "message": "Thinking..."
            }
        }))?;

        let value: wire::Notification<client::ClientEvent> = frame.try_into_notification()?.try_cast_into()?;
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

        let value: wire::Request<client::ClientParams> = frame.try_into_request()?.try_cast_into()?;
        let params = value.params.clone().try_into_connect()?;
        debug_assert_eq!(
            params.id,
            "019fb92c-e616-716f-9768-16c4753fe9d9".parse::<uuid::Uuid>().unwrap()
        );
        debug_assert_eq!(params.name, "test");
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

            let error = frame.clone().try_into_response()?.result().unwrap_err().clone();
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

            let res = frame.clone().try_into_response()?.result().unwrap().cloned();
            debug_assert_eq!(res, Some(serde_json::to_value(200)?), "{frame:#?}");
            let json = serde_json::to_string(&frame)?;
            debug_assert_eq!(
                json, r#"{"id":"019fb92c-e616-716f-9768-16c4753fe9d8","result":200}"#,
                "{json}"
            );

            Ok(())
        }
    }
}
