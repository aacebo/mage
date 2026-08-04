mod stream;

pub use stream::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum ClientEvent {
    Stream(StreamEvent),
}

impl ClientEvent {
    pub fn try_into_stream(self) -> Result<StreamEvent, Box<dyn std::error::Error>> {
        match self {
            Self::Stream(v) => Ok(v),
        }
    }
}

impl From<StreamEvent> for ClientEvent {
    fn from(value: StreamEvent) -> Self {
        Self::Stream(value)
    }
}
