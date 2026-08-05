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

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        match value.classify() {
            serde_json::error::Category::Syntax | serde_json::error::Category::Eof => parse(value),
            serde_json::error::Category::Data => invalid_request(value),
            serde_json::error::Category::Io => internal(value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructors_use_standard_codes_and_preserve_messages() {
        let cases = [
            (parse("parse failure"), Error::PARSE, "parse failure"),
            (invalid_request("invalid request"), Error::INVALID_REQUEST, "invalid request"),
            (method_not_found("unknown method"), Error::METHOD_NOT_FOUND, "unknown method"),
            (invalid_params("invalid params"), Error::INVALID_PARAMS, "invalid params"),
            (internal("internal failure"), Error::INTERNAL, "internal failure"),
        ];

        for (error, code, message) in cases {
            assert_eq!(error.code, code);
            assert_eq!(error.message, message);
            assert_eq!(error.to_string(), format!("{code} => {message}"));
        }
    }

    #[test]
    fn error_round_trips_with_extension_code() {
        let error = Error {
            code: -32001,
            message: "extension failure".to_string(),
        };

        let json = serde_json::to_string(&error).unwrap();
        assert_eq!(json, r#"{"code":-32001,"message":"extension failure"}"#);
        assert_eq!(serde_json::from_str::<Error>(&json).unwrap(), error);
    }

    #[test]
    fn json_errors_are_classified_by_category() {
        let syntax = serde_json::from_str::<serde_json::Value>("{").unwrap_err();
        assert_eq!(Error::from(syntax).code, Error::PARSE);

        let data = serde_json::from_value::<uuid::Uuid>(serde_json::json!(42)).unwrap_err();
        assert_eq!(Error::from(data).code, Error::INVALID_REQUEST);
    }
}
