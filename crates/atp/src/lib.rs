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

    pub async fn read<T>(&mut self) -> Result<Option<wire::Frame<T>>, Error>
    where
        T: for<'a> serde::Deserialize<'a>,
    {
        match self.socket.try_next().await? {
            Some(tungstenite::Message::Binary(bytes)) => Ok(serde_json::from_slice(&bytes)?),
            Some(tungstenite::Message::Text(text)) => Ok(serde_json::from_str(&text)?),
            Some(tungstenite::Message::Ping(_)) | Some(tungstenite::Message::Pong(_)) => todo!(),
            _ => Ok(None),
        }
    }
}

impl From<SocketStream> for Socket {
    fn from(socket: SocketStream) -> Self {
        Self { socket }
    }
}
