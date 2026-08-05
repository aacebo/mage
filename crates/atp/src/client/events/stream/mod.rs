use serde_valid::Validate;

mod activity;
mod close;
mod open;
mod text;

pub use activity::*;
pub use close::*;
pub use open::*;
pub use text::*;

use crate::error;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Validate)]
#[serde(untagged)]
pub enum StreamEvent {
    Close(StreamCloseEvent),
    Activity(StreamActivityEvent),
    Text(StreamTextEvent),
    Open(StreamOpenEvent),
}

impl StreamEvent {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Open(_) => "stream.open",
            Self::Activity(_) => "stream.activity",
            Self::Text(_) => "stream.text",
            Self::Close(_) => "stream.close",
        }
    }

    pub fn stream_id(&self) -> &str {
        match self {
            Self::Open(v) => &v.stream_id,
            Self::Close(v) => &v.stream_id,
            Self::Activity(v) => &v.stream_id,
            Self::Text(v) => &v.stream_id,
        }
    }

    pub fn sequence(&self) -> usize {
        match self {
            Self::Open(v) => v.sequence,
            Self::Activity(v) => v.sequence,
            Self::Text(v) => v.sequence,
            Self::Close(v) => v.sequence,
        }
    }

    pub fn try_into_open(self) -> crate::Result<StreamOpenEvent> {
        match self {
            Self::Open(v) => Ok(v),
            v => Err(error::invalid_request(format!(
                "expected `stream.open`, received `{}`",
                v.name()
            ))),
        }
    }

    pub fn try_into_activity(self) -> crate::Result<StreamActivityEvent> {
        match self {
            Self::Activity(v) => Ok(v),
            v => Err(error::invalid_request(format!(
                "expected `stream.status`, received `{}`",
                v.name()
            ))),
        }
    }

    pub fn try_into_text(self) -> crate::Result<StreamTextEvent> {
        match self {
            Self::Text(v) => Ok(v),
            v => Err(error::invalid_request(format!(
                "expected `stream.text`, received `{}`",
                v.name()
            ))),
        }
    }

    pub fn try_into_close(self) -> crate::Result<StreamCloseEvent> {
        match self {
            Self::Close(v) => Ok(v),
            v => Err(error::invalid_request(format!(
                "expected `stream.close`, received `{}`",
                v.name()
            ))),
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

impl From<StreamActivityEvent> for StreamEvent {
    fn from(value: StreamActivityEvent) -> Self {
        Self::Activity(value)
    }
}

impl From<StreamTextEvent> for StreamEvent {
    fn from(value: StreamTextEvent) -> Self {
        Self::Text(value)
    }
}
