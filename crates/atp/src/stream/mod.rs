use serde_valid::Validate;

mod close;
mod error;
mod open;
mod text;

pub use close::*;
pub use error::*;
pub use open::*;
pub use text::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Validate)]
#[serde(tag = "type")]
pub enum StreamEvent {
    #[serde(rename = "stream.open")]
    Open(StreamOpenEvent),

    #[serde(rename = "stream.close")]
    Close(StreamCloseEvent),

    #[serde(rename = "stream.error")]
    Error(StreamErrorEvent),

    #[serde(rename = "stream.text")]
    Text(StreamTextEvent),
}

impl StreamEvent {
    pub fn as_open(&self) -> Option<&StreamOpenEvent> {
        match self {
            Self::Open(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_close(&self) -> Option<&StreamCloseEvent> {
        match self {
            Self::Close(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_error(&self) -> Option<&StreamErrorEvent> {
        match self {
            Self::Error(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_text(&self) -> Option<&StreamTextEvent> {
        match self {
            Self::Text(v) => Some(v),
            _ => None,
        }
    }

    pub fn stream_id(&self) -> &str {
        match self {
            Self::Open(v) => &v.stream_id,
            Self::Close(v) => &v.stream_id,
            Self::Error(v) => &v.stream_id,
            Self::Text(v) => &v.stream_id,
        }
    }

    pub fn index(&self) -> Option<usize> {
        match self {
            Self::Error(v) => Some(v.index),
            Self::Text(v) => Some(v.index),
            _ => None,
        }
    }

    pub fn item(&self) -> Option<usize> {
        match self {
            Self::Error(v) => v.item,
            Self::Text(v) => v.item,
            _ => None,
        }
    }
}

impl From<StreamOpenEvent> for StreamEvent {
    fn from(value: StreamOpenEvent) -> Self {
        Self::Open(value)
    }
}

impl From<StreamCloseEvent> for StreamEvent {
    fn from(value: StreamCloseEvent) -> Self {
        Self::Close(value)
    }
}

impl From<StreamErrorEvent> for StreamEvent {
    fn from(value: StreamErrorEvent) -> Self {
        Self::Error(value)
    }
}

impl From<StreamTextEvent> for StreamEvent {
    fn from(value: StreamTextEvent) -> Self {
        Self::Text(value)
    }
}
