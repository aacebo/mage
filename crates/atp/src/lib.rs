use futures_util::SinkExt;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

pub mod client;
pub mod server;

pub type ClientWebSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub trait Producer {
    type Error;
    type Signal;

    fn send(&mut self, message: ProtocolMessage<Self::Signal>) -> impl Future<Output = Result<(), Self::Error>>;
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
    type Signal = client::Signal;

    async fn send(&mut self, message: ProtocolMessage<Self::Signal>) -> Result<(), Self::Error> {
        self.socket
            .send(tokio_tungstenite::tungstenite::Message::Binary(
                serde_json::to_vec(&message)?.into(),
            ))
            .await
            .map_err(mage_error::http)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use axum::Router;
    use axum::extract::State;
    use axum::extract::ws::{Message, WebSocketUpgrade};
    use axum::http::HeaderMap;
    use axum::response::Response;
    use axum::routing::get;
    use tokio::sync::mpsc;
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;

    use super::Producer;

    #[derive(Clone)]
    struct TestState {
        messages: mpsc::UnboundedSender<(HeaderMap, Message)>,
    }

    async fn connect(State(state): State<Arc<TestState>>, headers: HeaderMap, upgrade: WebSocketUpgrade) -> Response {
        upgrade.on_upgrade(move |mut socket| async move {
            if let Some(Ok(message)) = socket.recv().await {
                let _ = state.messages.send((headers, message));
            }
        })
    }

    #[tokio::test]
    async fn socket_connects_by_url_and_custom_request_and_sends_binary_json() {
        let (messages, mut received) = mpsc::unbounded_channel();
        let app = Router::new()
            .route("/connect", get(connect))
            .with_state(Arc::new(TestState { messages }));
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let url = format!("ws://{address}/connect");
        let mut socket = super::Socket::connect(url.as_str()).await.unwrap();
        let message = super::ProtocolMessage::new(uuid::Uuid::now_v7(), super::client::Signal::Ack);
        let expected = serde_json::to_vec(&message).unwrap();
        socket.send(message).await.unwrap();
        let (headers, frame) = tokio::time::timeout(Duration::from_secs(2), received.recv())
            .await
            .unwrap()
            .unwrap();

        assert!(headers.get("x-agent-id").is_none());
        assert_eq!(frame, Message::Binary(expected.into()));

        let mut request = url.into_client_request().unwrap();
        request
            .headers_mut()
            .insert("x-agent-id", "019c0000-0000-7000-8000-000000000000".parse().unwrap());
        let mut socket = super::Socket::connect(request).await.unwrap();
        let message = super::ProtocolMessage::new(uuid::Uuid::now_v7(), super::client::Signal::Ack);
        let expected = serde_json::to_vec(&message).unwrap();
        socket.send(message).await.unwrap();
        let (headers, frame) = tokio::time::timeout(Duration::from_secs(2), received.recv())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(headers["x-agent-id"], "019c0000-0000-7000-8000-000000000000");
        assert_eq!(frame, Message::Binary(expected.into()));
        assert!(
            super::Socket::connect(format!("ws://{address}/missing").as_str())
                .await
                .is_err()
        );
        server.abort();
    }
}
