mod events;
mod params;

pub use events::*;
pub use params::*;

use crate::wire;

pub trait Observe: Send {
    type Error: From<crate::Error> + Send;

    fn on_frame(
        &mut self,
        frame: wire::Frame<ClientFrame>,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + '_>> {
        Box::pin(async {
            match frame {
                wire::Frame::Notification(v) => {
                    let body = v.body.clone();
                    self.on_event(v.cast_with(body.try_into_event()?)).await
                }
                wire::Frame::Request(v) => {
                    let params = v.params.clone();
                    self.on_request(v.cast_with(params.try_into_params()?)).await
                }
                _ => Err(crate::error::invalid_request("expected client request or notification, received response").into()),
            }
        })
    }

    fn on_event(
        &mut self,
        event: wire::Notification<ClientEvent>,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + '_>> {
        Box::pin(async move {
            match event.body.clone() {
                ClientEvent::Stream(e) => self.on_stream_event(event.cast_with(e)).await,
            }
        })
    }

    fn on_request(
        &mut self,
        req: wire::Request<ClientParams>,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + '_>> {
        Box::pin(async move {
            match req.params.clone() {
                ClientParams::Connect(params) => self.on_connect_request(req.cast_with(params)).await,
                ClientParams::Message(params) => self.on_message_request(req.cast_with(params)).await,
            }
        })
    }

    fn on_connect_request(
        &mut self,
        _req: wire::Request<ConnectParams>,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + '_>> {
        Box::pin(async { Ok(()) })
    }

    fn on_message_request(
        &mut self,
        _req: wire::Request<MessageParams>,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + '_>> {
        Box::pin(async { Ok(()) })
    }

    fn on_stream_event(
        &mut self,
        event: wire::Notification<StreamEvent>,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + '_>> {
        Box::pin(async {
            match event.body.clone() {
                StreamEvent::Open(e) => self.on_stream_open_event(event.cast_with(e)).await,
                StreamEvent::Close(e) => self.on_stream_close_event(event.cast_with(e)).await,
                StreamEvent::Activity(e) => self.on_stream_activity_event(event.cast_with(e)).await,
                StreamEvent::Text(e) => self.on_stream_text_event(event.cast_with(e)).await,
            }
        })
    }

    fn on_stream_open_event(
        &mut self,
        _event: wire::Notification<StreamOpenEvent>,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + '_>> {
        Box::pin(async { Ok(()) })
    }

    fn on_stream_close_event(
        &mut self,
        _event: wire::Notification<StreamCloseEvent>,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + '_>> {
        Box::pin(async { Ok(()) })
    }

    fn on_stream_activity_event(
        &mut self,
        _event: wire::Notification<StreamActivityEvent>,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + '_>> {
        Box::pin(async { Ok(()) })
    }

    fn on_stream_text_event(
        &mut self,
        _event: wire::Notification<StreamTextEvent>,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + '_>> {
        Box::pin(async { Ok(()) })
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum ClientFrame {
    Event(ClientEvent),
    Params(ClientParams),
}

impl ClientFrame {
    pub fn try_into_event(self) -> crate::Result<ClientEvent> {
        match self {
            Self::Event(v) => Ok(v),
            _ => Err(crate::error::invalid_request("expected event")),
        }
    }

    pub fn try_into_params(self) -> crate::Result<ClientParams> {
        match self {
            Self::Params(v) => Ok(v),
            _ => Err(crate::error::invalid_request("expected request")),
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
        connects: usize,
        statuses: usize,
    }

    struct Rejecting;

    impl Observe for Rejecting {
        type Error = crate::Error;

        fn on_connect_request(
            &mut self,
            _req: wire::Request<ConnectParams>,
        ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + '_>> {
            Box::pin(async { Err(crate::error::internal("handler failed")) })
        }
    }

    impl Observe for Recorder {
        type Error = crate::Error;

        fn on_connect_request(
            &mut self,
            _req: wire::Request<ConnectParams>,
        ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + '_>> {
            Box::pin(async move {
                self.connects += 1;
                Ok(())
            })
        }

        fn on_stream_activity_event(
            &mut self,
            _event: wire::Notification<StreamActivityEvent>,
        ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + '_>> {
            Box::pin(async move {
                self.statuses += 1;
                Ok(())
            })
        }
    }

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
        assert_eq!(decoded.try_into_params().unwrap().try_into_connect().unwrap().id, agent_id);
    }

    #[test]
    fn observer_mutably_dispatches_typed_requests_and_events_with_send_futures() {
        let mut observer = Recorder::default();
        let connect = wire::Request {
            id: uuid::Uuid::now_v7(),
            method: "connect".to_string(),
            params: ClientFrame::from(ClientParams::from(ConnectParams {
                id: uuid::Uuid::now_v7(),
                name: "test".to_string(),
                description: "test agent".to_string(),
                secret: "secret".to_string(),
                skills: vec![],
            })),
        };
        let future = observer.on_frame(connect.into());
        assert_send(&future);
        block_on(future).unwrap();
        assert_eq!(observer.connects, 1);

        let status = wire::Notification {
            task_id: Some(uuid::Uuid::now_v7()),
            name: "stream.status".to_string(),
            body: ClientFrame::from(ClientEvent::from(StreamEvent::from(StreamActivityEvent {
                stream_id: "stream-1".to_string(),
                sequence: 1,
                phase: StreamPhase::Working,
                message: "working".to_string(),
            }))),
        };
        let future = observer.on_frame(status.into());
        assert_send(&future);
        block_on(future).unwrap();
        assert_eq!(observer.statuses, 1);
    }

    #[test]
    fn observer_propagates_handler_errors_and_rejects_responses() {
        let connect = wire::Request {
            id: uuid::Uuid::now_v7(),
            method: "connect".to_string(),
            params: ClientFrame::from(ClientParams::from(ConnectParams {
                id: uuid::Uuid::now_v7(),
                name: "test".to_string(),
                description: "test agent".to_string(),
                secret: "secret".to_string(),
                skills: vec![],
            })),
        };

        let mut observer = Rejecting;
        let error = block_on(observer.on_frame(connect.into())).unwrap_err();
        assert_eq!(error, crate::error::internal("handler failed"));

        let response = wire::Response::<ClientFrame>::Ok {
            id: uuid::Uuid::now_v7(),
            result: None,
        };
        let error = block_on(observer.on_frame(response.into())).unwrap_err();
        assert_eq!(error.code, crate::Error::INVALID_REQUEST);
    }
}
