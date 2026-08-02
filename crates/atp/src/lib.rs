use futures_util::{SinkExt, TryStreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, tungstenite};

pub mod client;
mod error;
pub mod server;
pub mod types;
pub mod wire;

pub use error::Error;

pub type SocketStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub struct Socket {
    socket: SocketStream,
}

impl Socket {
    pub async fn connect<R>(request: R) -> Result<Self, Error>
    where
        R: tungstenite::client::IntoClientRequest + Unpin,
    {
        let (socket, _) = tokio_tungstenite::connect_async(request).await?;
        Ok(Self { socket })
    }

    pub async fn write<T>(&mut self, frame: impl Into<wire::Frame<T>>) -> Result<(), Error>
    where
        T: serde::Serialize,
    {
        let frame = frame.into();

        Ok(self
            .socket
            .send(tungstenite::Message::Binary(serde_json::to_vec(&frame)?.into()))
            .await?)
    }

    pub async fn read<T>(&mut self) -> Result<Output<T>, Error>
    where
        T: for<'a> serde::Deserialize<'a>,
    {
        match self.socket.try_next().await? {
            Some(tungstenite::Message::Binary(bytes)) => Ok(Output::Frame(serde_json::from_slice(&bytes)?)),
            Some(tungstenite::Message::Text(text)) => Ok(Output::Frame(serde_json::from_str(&text)?)),
            Some(tungstenite::Message::Ping(_)) | Some(tungstenite::Message::Pong(_)) => Ok(Output::Continue),
            _ => Ok(Output::Close),
        }
    }
}

impl From<SocketStream> for Socket {
    fn from(socket: SocketStream) -> Self {
        Self { socket }
    }
}

#[derive(Debug, Clone)]
pub enum Output<T = serde_json::Value> {
    Frame(wire::Frame<T>),
    Continue,
    Close,
}

impl<T> Output<T> {
    pub fn is_frame(&self) -> bool {
        matches!(self, Self::Frame(_))
    }

    pub fn is_continue(&self) -> bool {
        matches!(self, Self::Continue)
    }

    pub fn is_close(&self) -> bool {
        matches!(self, Self::Close)
    }
}
