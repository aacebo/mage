pub mod session;

use std::collections::VecDeque;

use axum::extract::ws;
pub use session::*;

pub struct WebSocket {
    inner: ws::WebSocket,
    queue: VecDeque<ws::Message>,
}

impl WebSocket {
    pub fn close_with(
        &mut self,
        code: atp::CloseCode,
        reason: impl std::fmt::Display,
    ) -> std::pin::Pin<Box<impl Future<Output = Result<(), atp::Error>>>> {
        let reason = reason.to_string();

        Box::pin(async move {
            while let Some(message) = self.queue.pop_front() {
                self.inner.send(message).await.map_err(atp::error::socket)?;
            }

            self.inner
                .send(ws::Message::Close(Some(ws::CloseFrame {
                    code: code as u16,
                    reason: reason.into(),
                })))
                .await
                .map_err(atp::error::socket)
        })
    }
}

impl atp::Socket for WebSocket {
    type Error = atp::Error;
    type In = atp::client::ClientFrame;
    type Out = atp::server::ServerFrame;

    fn read(&mut self) -> std::pin::Pin<Box<impl Future<Output = Result<atp::Output<Self::In>, Self::Error>>>> {
        Box::pin(async move {
            match self.inner.recv().await {
                Some(Ok(ws::Message::Text(text))) => Ok(atp::Output::Frame(serde_json::from_str(text.as_str())?)),
                Some(Ok(ws::Message::Binary(bytes))) => Ok(atp::Output::Frame(serde_json::from_slice(&bytes)?)),
                Some(Ok(ws::Message::Ping(_) | ws::Message::Pong(_))) => Ok(atp::Output::Continue),
                Some(Ok(ws::Message::Close(Some(frame)))) => Ok(atp::Output::Close {
                    code: frame.code.try_into()?,
                    message: (!frame.reason.is_empty()).then(|| frame.reason.to_string()),
                }),
                Some(Ok(ws::Message::Close(None))) | None => Ok(atp::Output::Close {
                    code: atp::CloseCode::Normal,
                    message: None,
                }),
                Some(Err(error)) => Err(atp::error::socket(error)),
            }
        })
    }

    fn write(
        &mut self,
        item: impl Into<atp::wire::Frame<Self::Out>>,
    ) -> std::pin::Pin<Box<impl Future<Output = Result<(), Self::Error>>>> {
        let item = item.into();

        Box::pin(async move {
            let bytes = serde_json::to_vec(&item)?;
            self.queue.push_back(ws::Message::Binary(bytes.into()));
            Ok(())
        })
    }

    fn flush(&mut self) -> std::pin::Pin<Box<impl Future<Output = Result<usize, Self::Error>>>> {
        Box::pin(async move {
            let mut count = 0;

            while let Some(message) = self.queue.pop_front() {
                self.inner.send(message).await.map_err(atp::error::socket)?;
                count += 1;
            }

            Ok(count)
        })
    }

    fn close(&mut self) -> std::pin::Pin<Box<impl Future<Output = Result<(), Self::Error>>>> {
        Box::pin(async move {
            while let Some(message) = self.queue.pop_front() {
                self.inner.send(message).await.map_err(atp::error::socket)?;
            }

            self.inner
                .send(ws::Message::Close(Some(ws::CloseFrame {
                    code: atp::CloseCode::Normal as u16,
                    reason: "normal closure".into(),
                })))
                .await
                .map_err(atp::error::socket)
        })
    }
}

impl From<ws::WebSocket> for WebSocket {
    fn from(inner: ws::WebSocket) -> Self {
        Self {
            inner,
            queue: VecDeque::new(),
        }
    }
}
