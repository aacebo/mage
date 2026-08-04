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
    pub fn try_cast_into<T>(self) -> Result<Response<T>, crate::Error>
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Error {
    pub code: i64,
    pub message: String,
}

impl Error {
    pub const PARSE: i64 = -32700;
    pub const INVALID_REQUEST: i64 = -32600;
    pub const METHOD_NOT_FOUND: i64 = -32601;
    pub const INVALID_PARAMS: i64 = -32602;
    pub const INTERNAL: i64 = -32603;

    pub fn parse(message: impl std::fmt::Display) -> Self {
        Self {
            code: Self::PARSE,
            message: message.to_string(),
        }
    }

    pub fn invalid_request(message: impl std::fmt::Display) -> Self {
        Self {
            code: Self::INVALID_REQUEST,
            message: message.to_string(),
        }
    }

    pub fn method_not_found(message: impl std::fmt::Display) -> Self {
        Self {
            code: Self::METHOD_NOT_FOUND,
            message: message.to_string(),
        }
    }

    pub fn invalid_params(message: impl std::fmt::Display) -> Self {
        Self {
            code: Self::INVALID_PARAMS,
            message: message.to_string(),
        }
    }

    pub fn internal(message: impl std::fmt::Display) -> Self {
        Self {
            code: Self::INTERNAL,
            message: message.to_string(),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} => {}", self.code, self.message)
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
