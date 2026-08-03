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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connect_params_round_trip_through_client_frame() {
        let agent_id = uuid::Uuid::now_v7();
        let frame = ClientFrame::from(ClientParams::from(params::ConnectParams {
            id: agent_id,
            name: "test".to_string(),
            description: "test agent".to_string(),
            secret: "secret".to_string(),
            skills: vec![],
        }));

        let json = serde_json::to_string(&frame).unwrap();
        let decoded: ClientFrame = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.try_params().unwrap().try_connect().unwrap().id, agent_id);
    }
}
