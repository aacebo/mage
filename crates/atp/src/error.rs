pub fn parse(message: impl std::fmt::Display) -> Error {
    Error {
        code: Error::PARSE,
        message: message.to_string(),
    }
}

pub fn invalid_request(message: impl std::fmt::Display) -> Error {
    Error {
        code: Error::INVALID_REQUEST,
        message: message.to_string(),
    }
}

pub fn method_not_found(message: impl std::fmt::Display) -> Error {
    Error {
        code: Error::METHOD_NOT_FOUND,
        message: message.to_string(),
    }
}

pub fn invalid_params(message: impl std::fmt::Display) -> Error {
    Error {
        code: Error::INVALID_PARAMS,
        message: message.to_string(),
    }
}

pub fn internal(message: impl std::fmt::Display) -> Error {
    Error {
        code: Error::INTERNAL,
        message: message.to_string(),
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
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} => {}", self.code, self.message)
    }
}

impl std::error::Error for Error {}
