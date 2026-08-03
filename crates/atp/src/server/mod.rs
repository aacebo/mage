pub mod events;

pub use events::ServerEvent;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum ServerFrame {
    Event(ServerEvent),
}

impl ServerFrame {
    pub fn try_event(&self) -> Result<&ServerEvent, crate::Error> {
        match self {
            Self::Event(v) => Ok(v),
            #[allow(unused)]
            _ => Err(crate::error::protocol("expected server event frame")),
        }
    }
}

impl From<ServerEvent> for ServerFrame {
    fn from(value: ServerEvent) -> Self {
        Self::Event(value)
    }
}
