mod events;
mod params;

pub use events::*;
pub use params::*;

use crate::{error, wire};

pub trait Observe {
    fn on_frame(
        &self,
        frame: wire::Frame<ClientFrame>,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), crate::Error>> + '_>> {
        Box::pin(async {
            match frame {
                wire::Frame::Notification(v) => {
                    let body = v.body.clone();
                    self.on_event(v.cast_with(body.try_event()?.clone())).await
                }
                wire::Frame::Request(v) => {
                    let params = v.params.clone();
                    self.on_request(v.cast_with(params.try_params()?.clone())).await
                }
                wire::Frame::Response(_) => Err(error::protocol("unsupported client response")),
            }
        })
    }

    fn on_event(
        &self,
        event: wire::Notification<ClientEvent>,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), crate::Error>> + '_>> {
        Box::pin(async move {
            match event.body.clone() {
                ClientEvent::Stream(e) => self.on_stream_event(event.cast_with(e)).await,
            }
        })
    }

    fn on_request(
        &self,
        req: wire::Request<ClientParams>,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), crate::Error>> + '_>> {
        Box::pin(async move {
            match req.params.clone() {
                ClientParams::Connect(params) => self.on_connect_request(req.cast_with(params)).await,
                ClientParams::Message(params) => self.on_message_request(req.cast_with(params)).await,
            }
        })
    }

    fn on_connect_request(
        &self,
        _req: wire::Request<ConnectParams>,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), crate::Error>>>> {
        Box::pin(async { Ok(()) })
    }

    fn on_message_request(
        &self,
        _req: wire::Request<MessageParams>,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), crate::Error>>>> {
        Box::pin(async { Ok(()) })
    }

    fn on_stream_event(
        &self,
        event: wire::Notification<StreamEvent>,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), crate::Error>> + '_>> {
        Box::pin(async {
            match event.body.clone() {
                StreamEvent::Open(e) => self.on_stream_open_event(event.cast_with(e)).await,
                StreamEvent::Close(e) => self.on_stream_close_event(event.cast_with(e)).await,
                StreamEvent::Status(e) => self.on_stream_status_event(event.cast_with(e)).await,
                StreamEvent::Text(e) => self.on_stream_text_event(event.cast_with(e)).await,
            }
        })
    }

    fn on_stream_open_event(
        &self,
        _event: wire::Notification<StreamOpenEvent>,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), crate::Error>>>> {
        Box::pin(async { Ok(()) })
    }

    fn on_stream_close_event(
        &self,
        _event: wire::Notification<StreamCloseEvent>,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), crate::Error>>>> {
        Box::pin(async { Ok(()) })
    }

    fn on_stream_status_event(
        &self,
        _event: wire::Notification<StreamStatusEvent>,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), crate::Error>>>> {
        Box::pin(async { Ok(()) })
    }

    fn on_stream_text_event(
        &self,
        _event: wire::Notification<StreamTextEvent>,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), crate::Error>>>> {
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
