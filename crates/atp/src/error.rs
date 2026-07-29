#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "code", content = "message")]
pub enum Error {
    Protocol(String),
    Json(String),
    Socket(String),
}

impl Error {
    pub fn code(&self) -> &str {
        match self {
            Self::Protocol(_) => "protocol",
            Self::Json(_) => "json",
            Self::Socket(_) => "socket",
        }
    }

    pub fn message(&self) -> &str {
        match self {
            Self::Protocol(v) => v.as_str(),
            Self::Json(v) => v.as_str(),
            Self::Socket(v) => v.as_str(),
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value.to_string())
    }
}

impl From<tokio_tungstenite::tungstenite::Error> for Error {
    fn from(value: tokio_tungstenite::tungstenite::Error) -> Self {
        Self::Socket(value.to_string())
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} => {}", self.code(), self.message())
    }
}

impl std::error::Error for Error {}
