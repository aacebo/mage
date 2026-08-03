pub mod events;
pub mod params;

pub use events::ClientEvent;
pub use params::ClientParams;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum ClientFrame {
    Event(ClientEvent),
    Params(ClientParams),
}

impl ClientFrame {
    pub fn try_event(&self) -> Result<&ClientEvent, crate::Error> {
        match self {
            Self::Event(v) => Ok(v),
            _ => Err(crate::error::protocol("expected client event frame")),
        }
    }

    pub fn try_params(&self) -> Result<&ClientParams, crate::Error> {
        match self {
            Self::Params(v) => Ok(v),
            _ => Err(crate::error::protocol("expected client params frame")),
        }
    }
}

impl From<ClientEvent> for ClientFrame {
    fn from(value: ClientEvent) -> Self {
        Self::Event(value)
    }
}

impl From<ClientParams> for ClientFrame {
    fn from(value: ClientParams) -> Self {
        Self::Params(value)
    }
}
