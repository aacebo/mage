use serde_valid::Validate;

mod close;
mod open;
mod status;
mod text;

pub use close::*;
pub use open::*;
pub use status::*;
pub use text::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Validate)]
#[serde(untagged)]
pub enum StreamEvent {
    Open(StreamOpenEvent),
    Close(StreamCloseEvent),
    Status(StreamStatusEvent),
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

    pub fn as_status(&self) -> Option<&StreamStatusEvent> {
        match self {
            Self::Status(v) => Some(v),
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
            Self::Status(v) => &v.stream_id,
            Self::Text(v) => &v.stream_id,
        }
    }

    pub fn sequence(&self) -> usize {
        match self {
            Self::Open(v) => v.sequence,
            Self::Status(v) => v.sequence,
            Self::Text(v) => v.sequence,
            Self::Close(v) => v.sequence,
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

impl From<StreamStatusEvent> for StreamEvent {
    fn from(value: StreamStatusEvent) -> Self {
        Self::Status(value)
    }
}

impl From<StreamTextEvent> for StreamEvent {
    fn from(value: StreamTextEvent) -> Self {
        Self::Text(value)
    }
}
