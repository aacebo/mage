mod events;

pub use events::*;

use crate::{error, wire};

pub trait Observe {
    fn on_frame(
        &self,
        frame: wire::Frame<ServerFrame>,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), crate::Error>> + '_>> {
        Box::pin(async {
            match frame {
                wire::Frame::Notification(v) => {
                    let body = v.body.clone();
                    self.on_event(v.cast_with(body.try_event()?.clone())).await
                }
                wire::Frame::Request(v) => Err(error::protocol(format!("unsupported client request => {v:#?}"))),
                wire::Frame::Response(v) => Err(error::protocol(format!("unsupported client response => {v:#?}"))),
            }
        })
    }

    fn on_event(
        &self,
        event: wire::Notification<ServerEvent>,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), crate::Error>> + '_>> {
        Box::pin(async {
            match event.body.clone() {
                ServerEvent::Message(e) => self.on_message_event(event.cast_with(e)).await,
            }
        })
    }

    fn on_message_event(
        &self,
        _event: wire::Notification<MessageEvent>,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), crate::Error>> + '_>> {
        Box::pin(async { Ok(()) })
    }
}

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
