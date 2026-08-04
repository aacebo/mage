use crate::Error;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum Response<T = serde_json::Value> {
    Err {
        id: uuid::Uuid,
        error: Error,
    },
    Ok {
        id: uuid::Uuid,

        #[serde(skip_serializing_if = "Option::is_none")]
        result: Option<T>,
    },
}

impl<T> Response<T> {
    pub fn id(&self) -> uuid::Uuid {
        match self {
            Self::Err { id, error: _ } => *id,
            Self::Ok { id, result: _ } => *id,
        }
    }

    pub fn result(&self) -> Result<Option<&T>, &Error> {
        match self {
            Self::Err { id: _, error } => Err(error),
            Self::Ok { id: _, result } => match result {
                Some(result) => Ok(Some(result)),
                None => Ok(None),
            },
        }
    }

    pub fn cast_with<V>(self, result: Option<V>) -> Response<V> {
        match self {
            Self::Err { id, error } => Response::Err { id, error },
            Self::Ok { id, result: _ } => Response::Ok { id, result },
        }
    }
}

impl Response {
    pub fn try_cast_into<T>(self) -> Result<Response<T>, Box<dyn std::error::Error>>
    where
        T: for<'a> serde::Deserialize<'a>,
    {
        match self {
            Self::Err { id, error } => Ok(Response::Err { id, error }),
            Self::Ok { id, result } => Ok(Response::Ok {
                id,
                result: match result {
                    None => None,
                    Some(result) => serde_json::from_value(result)?,
                },
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Error, wire};

    mod serde {
        use super::*;

        #[test]
        fn error() -> Result<(), Error> {
            let frame: wire::Response = serde_json::from_value(serde_json::json!({
                "id": "019fb92c-e616-716f-9768-16c4753fe9d8",
                "error": {
                    "code": -200,
                    "message": "??"
                }
            }))?;

            let error = frame.result().unwrap_err();
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
            let frame: wire::Response = serde_json::from_value(serde_json::json!({
                "id": "019fb92c-e616-716f-9768-16c4753fe9d8"
            }))?;

            let res = frame.result().unwrap();
            debug_assert!(res.is_none(), "{frame:#?}");
            let json = serde_json::to_string(&frame)?;
            debug_assert_eq!(json, r#"{"id":"019fb92c-e616-716f-9768-16c4753fe9d8"}"#, "{json}");

            Ok(())
        }
    }
}
