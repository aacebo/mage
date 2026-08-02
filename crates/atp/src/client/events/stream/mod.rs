use serde_valid::Validate;

mod close;
mod open;
mod status;
mod text;

pub use close::*;
pub use open::*;
pub use status::*;
pub use text::*;

use crate::{Error, error};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Validate)]
#[serde(untagged)]
pub enum StreamEvent {
    Close(StreamCloseEvent),
    Status(StreamStatusEvent),
    Text(StreamTextEvent),
    Open(StreamOpenEvent),
}

impl StreamEvent {
    pub fn try_open(&self) -> Result<&StreamOpenEvent, Error> {
        match self {
            Self::Open(v) => Ok(v),
            _ => Err(error::protocol("expected `stream.open` event")),
        }
    }

    pub fn try_status(&self) -> Result<&StreamStatusEvent, Error> {
        match self {
            Self::Status(v) => Ok(v),
            _ => Err(error::protocol("expected `stream.status` event")),
        }
    }

    pub fn try_text(&self) -> Result<&StreamTextEvent, Error> {
        match self {
            Self::Text(v) => Ok(v),
            _ => Err(error::protocol("expected `stream.text` event")),
        }
    }

    pub fn try_close(&self) -> Result<&StreamCloseEvent, Error> {
        match self {
            Self::Close(v) => Ok(v),
            _ => Err(error::protocol("expected `stream.close` event")),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Open(_) => "stream.open",
            Self::Status(_) => "stream.status",
            Self::Text(_) => "stream.text",
            Self::Close(_) => "stream.close",
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
