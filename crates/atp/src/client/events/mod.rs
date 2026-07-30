mod stream;

pub use stream::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "name", content = "body", rename_all = "snake_case")]
pub enum ClientEvent {
    #[serde(untagged)]
    Stream(StreamEvent),
}

impl ClientEvent {
    pub fn as_stream(&self) -> Option<&StreamEvent> {
        match self {
            Self::Stream(v) => Some(v),
            #[allow(unused)]
            _ => None,
        }
    }
}

impl From<StreamEvent> for ClientEvent {
    fn from(value: StreamEvent) -> Self {
        Self::Stream(value)
    }
}
