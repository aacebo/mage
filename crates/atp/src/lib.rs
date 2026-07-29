use futures_util::SinkExt;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

pub mod client;
pub mod server;

pub type ClientWebSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub trait Producer {
    type Error;

    fn send(&mut self, frame: Frame) -> impl Future<Output = Result<(), Self::Error>>;
}

pub trait Consumer {
    type Error;

    fn recv(&mut self) -> impl Future<Output = Result<Frame, Self::Error>>;
}

pub struct Socket {
    socket: ClientWebSocket,
}

impl Socket {
    pub async fn connect<R>(request: R) -> mage_error::Result<Self>
    where
        R: IntoClientRequest + Unpin,
    {
        let (socket, _) = tokio_tungstenite::connect_async(request).await.map_err(mage_error::http)?;
        Ok(Self { socket })
    }
}

impl From<ClientWebSocket> for Socket {
    fn from(socket: ClientWebSocket) -> Self {
        Self { socket }
    }
}

impl Producer for Socket {
    type Error = mage_error::Error;

    async fn send(&mut self, frame: Frame) -> Result<(), Self::Error> {
        self.socket
            .send(tokio_tungstenite::tungstenite::Message::Binary(
                serde_json::to_vec(&frame)?.into(),
            ))
            .await
            .map_err(mage_error::http)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Frame {
    Request(Request),
    Response(Response),
    Event(Event),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Request {
    pub id: uuid::Uuid,
    pub method: String,
    pub params: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Response {
    Ok { id: uuid::Uuid, result: serde_json::Value },
    Err { id: uuid::Uuid, error: Error },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Event {
    pub task_id: Option<uuid::Uuid>,
    pub sequence: Option<u64>,
    pub body: serde_json::Value,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Error {
    pub code: i64,
    pub message: String,
}

impl Error {
    pub const PARSE: i64 = -32700;
    pub const INVALID_REQUEST: i64 = -32600;
    pub const METHOD_NOT_FOUND: i64 = -32601;
    pub const INVALID_PARAMS: i64 = -32602;
    pub const INTERNAL: i64 = -32603;

    pub fn parse(message: impl std::fmt::Display) -> Self {
        Self {
            code: Self::PARSE,
            message: message.to_string(),
        }
    }

    pub fn invalid_request(message: impl std::fmt::Display) -> Self {
        Self {
            code: Self::INVALID_REQUEST,
            message: message.to_string(),
        }
    }

    pub fn method_not_found(message: impl std::fmt::Display) -> Self {
        Self {
            code: Self::METHOD_NOT_FOUND,
            message: message.to_string(),
        }
    }

    pub fn invalid_params(message: impl std::fmt::Display) -> Self {
        Self {
            code: Self::INVALID_PARAMS,
            message: message.to_string(),
        }
    }

    pub fn internal(message: impl std::fmt::Display) -> Self {
        Self {
            code: Self::INTERNAL,
            message: message.to_string(),
        }
    }
}
