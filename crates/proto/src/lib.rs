use actix_codec::Framed;

pub mod client;
pub mod server;

pub trait Producer {
    type Error;
    type Signal;

    fn send(&mut self, message: ProtocolMessage<Self::Signal>) -> impl Future<Output = Result<(), Self::Error>> + Send;
}

pub trait Consumer {
    type Error;
    type Signal;

    fn recv(&mut self) -> impl Future<Output = Result<ProtocolMessage<Self::Signal>, Self::Error>> + Send;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProtocolMessage<T> {
    pub id: uuid::Uuid,
    pub trace_id: uuid::Uuid,
    pub reply_to_id: Option<uuid::Uuid>,
    pub body: T,
    pub sent_at: chrono::DateTime<chrono::Utc>,
}

impl<T> ProtocolMessage<T> {
    pub fn new(trace_id: uuid::Uuid, body: impl Into<T>) -> Self {
        Self {
            id: uuid::Uuid::now_v7(),
            trace_id,
            reply_to_id: None,
            body: body.into(),
            sent_at: chrono::Utc::now(),
        }
    }

    pub fn reply<V>(&self, body: V) -> ProtocolMessage<V> {
        ProtocolMessage {
            id: uuid::Uuid::now_v7(),
            trace_id: self.trace_id,
            reply_to_id: Some(self.id),
            body,
            sent_at: chrono::Utc::now(),
        }
    }
}

pub struct Socket {
    socket: std::pin::Pin<Framed<awc::BoxedSocket, awc::ws::Codec>>,
}

impl From<Framed<awc::BoxedSocket, awc::ws::Codec>> for Socket {
    fn from(socket: Framed<awc::BoxedSocket, awc::ws::Codec>) -> Self {
        Self { socket: socket.into() }
    }
}

impl Producer for Socket {
    type Error = error::Error;
    type Signal = client::Signal;

    fn send(&mut self, message: ProtocolMessage<Self::Signal>) -> impl Future<Output = Result<(), Self::Error>> + Send {
        self.socket
    }
}
