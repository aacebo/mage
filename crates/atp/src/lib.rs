#[doc(inline)]
pub use error::Error;
#[doc(inline)]
pub use frame::Frame;
use futures_util::SinkExt;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

// pub mod client;
mod error;
mod frame;
// pub mod server;

pub type SocketStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub trait Producer {
    type Error;

    fn send(&mut self, frame: Frame) -> impl Future<Output = Result<(), Self::Error>>;
}

pub trait Consumer {
    type Error;

    fn recv(&mut self) -> impl Future<Output = Result<Frame, Self::Error>>;
}

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
}

impl From<SocketStream> for Socket {
    fn from(socket: SocketStream) -> Self {
        Self { socket }
    }
}

impl Producer for Socket {
    type Error = Error;

    async fn send(&mut self, frame: Frame) -> Result<(), Self::Error> {
        Ok(self
            .socket
            .send(tokio_tungstenite::tungstenite::Message::Binary(
                serde_json::to_vec(&frame)?.into(),
            ))
            .await?)
    }
}
