mod events;

pub use events::*;

use crate::{error, wire};

pub trait Observe: Send {
    fn on_frame(
        &mut self,
        frame: wire::Frame<ServerFrame>,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error>>> + Send + '_>> {
        Box::pin(async {
            match frame {
                wire::Frame::Notification(v) => {
                    let body = v.body.clone();
                    self.on_event(v.cast_with(body.try_event()?.clone())).await
                }
                _ => Err(error::invalid_request("expected notification, received request or response").into()),
            }
        })
    }

    fn on_event(
        &mut self,
        event: wire::Notification<ServerEvent>,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error>>> + Send + '_>> {
        Box::pin(async {
            match event.body.clone() {
                ServerEvent::Message(e) => self.on_message_event(event.cast_with(e)).await,
            }
        })
    }

    fn on_message_event(
        &mut self,
        _event: wire::Notification<MessageEvent>,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error>>> + Send + '_>> {
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
    use std::future::Future;
    use std::task::{Context, Poll, Waker};

    use super::*;

    fn block_on<F: Future>(future: F) -> F::Output {
        let waker = Waker::noop();
        let mut context = Context::from_waker(waker);
        let mut future = std::pin::pin!(future);

        loop {
            match future.as_mut().poll(&mut context) {
                Poll::Ready(output) => return output,
                Poll::Pending => std::thread::yield_now(),
            }
        }
    }

    fn assert_send<T: Send>(_: &T) {}

    #[derive(Default)]
    struct Recorder {
        messages: usize,
    }

    impl Observe for Recorder {
        fn on_message_event(
            &mut self,
            _event: wire::Notification<MessageEvent>,
        ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), crate::Error>> + Send + '_>> {
            Box::pin(async move {
                self.messages += 1;
                Ok(())
            })
        }
    }

    fn message_event() -> MessageEvent {
        let now = chrono::Utc::now();
        MessageEvent {
            id: uuid::Uuid::now_v7(),
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
        }
    }

    #[test]
    fn message_event_round_trips_through_server_frame() {
        let event = message_event();
        let message_id = event.id;
        let frame = ServerFrame::from(ServerEvent::from(event));

        let json = serde_json::to_string(&frame).unwrap();
        let decoded: ServerFrame = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.try_event().unwrap().try_message().unwrap().id, message_id);
    }

    #[test]
    fn observer_mutably_dispatches_message_events_with_send_futures() {
        let mut observer = Recorder::default();
        let notification = wire::Notification {
            task_id: Some(uuid::Uuid::now_v7()),
            name: "message".to_string(),
            body: ServerFrame::from(ServerEvent::from(message_event())),
        };

        let future = observer.on_frame(notification.into());
        assert_send(&future);
        block_on(future).unwrap();
        assert_eq!(observer.messages, 1);
    }
}
