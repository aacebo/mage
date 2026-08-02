mod stream;

pub use stream::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum ClientEvent {
    Stream(StreamEvent),
}

impl ClientEvent {
    pub fn try_stream(&self) -> Result<&StreamEvent, crate::Error> {
        match self {
            Self::Stream(v) => Ok(v),
            #[allow(unused)]
            _ => Err(crate::error::protocol("expected stream event")),
        }
    }
}

impl From<StreamEvent> for ClientEvent {
    fn from(value: StreamEvent) -> Self {
        Self::Stream(value)
    }
}
