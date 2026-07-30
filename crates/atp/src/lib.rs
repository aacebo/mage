use futures_util::{SinkExt, TryStreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

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
        R: IntoClientRequest + Unpin,
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
            .send(tokio_tungstenite::tungstenite::Message::Binary(
                serde_json::to_vec(&frame)?.into(),
            ))
            .await?)
    }

    pub async fn read<T>(&mut self) -> Result<Option<wire::Frame<T>>, Error>
    where
        T: for<'a> serde::Deserialize<'a>,
    {
        if let Some(tokio_tungstenite::tungstenite::Message::Binary(bytes)) = self.socket.try_next().await? {
            Ok(serde_json::from_slice(&bytes)?)
        } else {
            Ok(None)
        }
    }
}

impl From<SocketStream> for Socket {
    fn from(socket: SocketStream) -> Self {
        Self { socket }
    }
}
