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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    #[test]
    fn message_event_round_trips_through_server_frame() {
        let message_id = uuid::Uuid::now_v7();
        let now = chrono::Utc::now();
        let frame = ServerFrame::from(ServerEvent::from(events::MessageEvent {
            id: message_id,
            chat: crate::types::Chat {
                id: uuid::Uuid::now_v7(),
                tenant_id: uuid::Uuid::now_v7(),
                name: None,
            },
            content: vec![crate::types::Content::Text {
                text: "hello".to_string(),
            }],
            metadata: BTreeMap::new(),
            created_by: crate::types::Actor {
                id: uuid::Uuid::now_v7(),
                role: crate::types::Role::Agent,
                name: "agent".to_string(),
            },
            created_at: now,
            updated_at: now,
        }));

        let json = serde_json::to_string(&frame).unwrap();
        let decoded: ServerFrame = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.try_event().unwrap().try_message().unwrap().id, message_id);
    }
}
