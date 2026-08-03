use std::collections::VecDeque;

use axum::extract::ws;

pub struct WebSocket {
    inner: ws::WebSocket,
    queue: VecDeque<ws::Message>,
}

impl WebSocket {
    pub fn panic(&mut self, reason: impl std::fmt::Display) -> std::pin::Pin<Box<impl Future<Output = Result<(), atp::Error>>>> {
        let reason = reason.to_string();

        Box::pin(async {
            self.inner
                .send(ws::Message::Close(Some(ws::CloseFrame {
                    code: 1,
                    reason: reason.into(),
                })))
                .await
                .map_err(atp::error::protocol)?;
            Ok(())
        })
    }
}

impl atp::Socket for WebSocket {
    type Error = atp::Error;
    type In = atp::client::ClientFrame;
    type Out = atp::server::ServerFrame;

    fn read(&mut self) -> std::pin::Pin<Box<impl Future<Output = Result<atp::Output<Self::In>, Self::Error>>>> {
        Box::pin(async {
            match self.inner.recv().await {
                Some(Ok(ws::Message::Close(Some(frame)))) => Ok(atp::Output::Close {
                    code: frame.code,
                    message: Some(frame.reason.to_string()),
                }),
                _ => Ok(atp::Output::Close { code: 0, message: None }),
            }
        })
    }

    fn write(
        &mut self,
        item: impl Into<atp::wire::Frame<Self::Out>>,
    ) -> std::pin::Pin<Box<impl Future<Output = Result<(), Self::Error>>>> {
        Box::pin(async {
            let item = item.into();
            let bytes = serde_json::to_vec(&item)?;
            self.inner
                .send(ws::Message::Binary(bytes.into()))
                .await
                .map_err(atp::error::protocol)
        })
    }

    fn flush(&mut self) -> std::pin::Pin<Box<impl Future<Output = Result<usize, Self::Error>>>> {
        Box::pin(async {
            let mut i = 0;

            while let Some(message) = self.queue.pop_front() {
                i += 1;
                self.inner.send(message).await.map_err(atp::error::protocol)?;
            }

            Ok(i)
        })
    }

    fn close(&mut self) -> std::pin::Pin<Box<impl Future<Output = Result<(), Self::Error>>>> {
        Box::pin(async {
            self.inner
                .send(ws::Message::Close(None))
                .await
                .map_err(atp::error::protocol)?;
            Ok(())
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
