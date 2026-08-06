pub mod session;

use axum::extract::ws;
pub use session::*;

pub struct WebSocket {
    inner: ws::WebSocket,
}

impl atp::Socket for WebSocket {
    type Error = mage_error::Error;

    fn read<T>(&mut self) -> std::pin::Pin<Box<impl Future<Output = Result<atp::Output<T>, Self::Error>>>>
    where
        T: for<'a> serde::Deserialize<'a>,
    {
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
                Some(Err(error)) => Err(mage_error::internal(error)),
            }
        })
    }

    fn write<T>(&mut self, item: T) -> std::pin::Pin<Box<impl Future<Output = Result<(), Self::Error>>>>
    where
        T: serde::Serialize,
    {
        Box::pin(async move {
            let bytes = serde_json::to_vec(&item)?;
            self.inner
                .send(ws::Message::Binary(bytes.into()))
                .await
                .map_err(mage_error::internal)?;
            Ok(())
        })
    }

    fn close(
        &mut self,
        code: atp::CloseCode,
        reason: Option<impl std::fmt::Display>,
    ) -> std::pin::Pin<Box<impl Future<Output = Result<(), Self::Error>>>> {
        Box::pin(async move {
            self.inner
                .send(ws::Message::Close(Some(ws::CloseFrame {
                    code: code as u16,
                    reason: reason.map_or("normal closure".to_string(), |v| v.to_string()).into(),
                })))
                .await
                .map_err(mage_error::internal)
        })
    }
}

impl From<ws::WebSocket> for WebSocket {
    fn from(inner: ws::WebSocket) -> Self {
        Self { inner }
    }
}

impl std::ops::Deref for WebSocket {
    type Target = ws::WebSocket;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl std::ops::DerefMut for WebSocket {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
