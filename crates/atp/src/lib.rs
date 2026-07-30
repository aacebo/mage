#[doc(inline)]
pub use error::Error;
#[doc(inline)]
pub use frame::Frame;
use futures_util::SinkExt;
#[doc(inline)]
pub use message::Message;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

// pub mod client;
mod connect;
mod error;
pub mod frame;
mod message;
pub mod stream;
// pub mod server;

pub type SocketStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub trait Producer {
    type Error;

    fn send<T>(&mut self, frame: Frame<T>) -> impl Future<Output = Result<(), Self::Error>>
    where
        T: serde::Serialize;
}

pub trait Consumer {
    type Error;

    fn recv<T>(&mut self) -> impl Future<Output = Result<Frame, Self::Error>>
    where
        T: for<'a> serde::Deserialize<'a>;
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

    async fn send<T>(&mut self, frame: Frame<T>) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        Ok(self
            .socket
            .send(tokio_tungstenite::tungstenite::Message::Binary(
                serde_json::to_vec(&frame)?.into(),
            ))
            .await?)
    }
}
