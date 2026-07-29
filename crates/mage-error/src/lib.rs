pub trait IntoError {
    fn into_error(self) -> Error;
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn new(name: impl std::fmt::Display, message: impl std::fmt::Display) -> Error {
    Error {
        trace_id: None,
        name: name.to_string(),
        message: message.to_string(),
    }
}

pub fn amqp(message: impl std::fmt::Display) -> Error {
    new("amqp", message)
}

pub fn io(message: impl std::fmt::Display) -> Error {
    new("io", message)
}

pub fn parse(message: impl std::fmt::Display) -> Error {
    new("parse", message)
}

pub fn sql(message: impl std::fmt::Display) -> Error {
    new("sql", message)
}

pub fn http(message: impl std::fmt::Display) -> Error {
    new("http", message)
}

pub fn json(message: impl std::fmt::Display) -> Error {
    new("json", message)
}

pub fn bad_request(message: impl std::fmt::Display) -> Error {
    new("bad_request", message)
}

pub fn unauthorized(message: impl std::fmt::Display) -> Error {
    new("unauthorized", message)
}

pub fn config(message: impl std::fmt::Display) -> Error {
    new("config", message)
}

pub fn ai(message: impl std::fmt::Display) -> Error {
    new("ai", message)
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Error {
    trace_id: Option<String>,
    name: String,
    message: String,
}

impl Error {
    pub fn trace(mut self, value: impl std::fmt::Display) -> Self {
        self.trace_id = Some(value.to_string());
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "mage_error::{} => {}", self.name, self.message)
    }
}

impl<T: Into<Error>> IntoError for T {
    fn into_error(self) -> Error {
        self.into()
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        json(value)
    }
}

impl<T: std::error::Error + 'static> From<serde_valid::Error<T>> for Error {
    fn from(value: serde_valid::Error<T>) -> Self {
        bad_request(value)
    }
}

impl From<serde_valid::validation::Error> for Error {
    fn from(value: serde_valid::validation::Error) -> Self {
        bad_request(value)
    }
}

impl From<serde_valid::validation::Errors> for Error {
    fn from(value: serde_valid::validation::Errors) -> Self {
        bad_request(value)
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        io(value)
    }
}

#[cfg(feature = "amqp")]
impl From<lapin::Error> for Error {
    fn from(value: lapin::Error) -> Self {
        amqp(value)
    }
}

#[cfg(feature = "ai")]
impl From<reqwest::Error> for Error {
    fn from(value: reqwest::Error) -> Self {
        ai(value)
    }
}

#[cfg(feature = "ai")]
impl From<url::ParseError> for Error {
    fn from(value: url::ParseError) -> Self {
        ai(value)
    }
}

#[cfg(feature = "storage")]
impl From<sqlx::Error> for Error {
    fn from(value: sqlx::Error) -> Self {
        sql(value)
    }
}

#[cfg(feature = "web")]
impl Error {
    pub fn status_code(&self) -> axum::http::StatusCode {
        match self.name() {
            "not_found" => axum::http::StatusCode::NOT_FOUND,
            "bad_request" | "parse" | "json" => axum::http::StatusCode::BAD_REQUEST,
            "unauthorized" => axum::http::StatusCode::UNAUTHORIZED,
            _ => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[cfg(feature = "web")]
impl axum::response::IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        (self.status_code(), axum::Json(self)).into_response()
    }
}

#[cfg(all(test, feature = "web"))]
mod tests {
    use axum::body::to_bytes;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    #[tokio::test]
    async fn web_errors_preserve_status_and_json() {
        let cases = [
            (super::new("not_found", "missing"), StatusCode::NOT_FOUND),
            (super::bad_request("bad"), StatusCode::BAD_REQUEST),
            (super::unauthorized("denied"), StatusCode::UNAUTHORIZED),
            (super::http("failed"), StatusCode::INTERNAL_SERVER_ERROR),
        ];

        for (error, expected_status) in cases {
            let expected = serde_json::to_value(&error).unwrap();
            let response = error.into_response();
            assert_eq!(response.status(), expected_status);
            assert_eq!(response.headers()[axum::http::header::CONTENT_TYPE], "application/json");
            let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
            assert_eq!(serde_json::from_slice::<serde_json::Value>(&body).unwrap(), expected);
        }
    }
}
