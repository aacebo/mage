#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum Response<T = serde_json::Value> {
    Ok { id: uuid::Uuid, result: T },
    Err { id: uuid::Uuid, error: Error },
}

impl<T> Response<T> {
    pub fn payload(&self) -> Result<&T, &Error> {
        match self {
            Self::Ok { id: _, result } => Ok(result),
            Self::Err { id: _, error } => Err(error),
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
                result: serde_json::from_value(result)?,
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
