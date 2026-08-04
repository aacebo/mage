use std::time::Duration;

mod actor;
mod session;

use atp::Socket;
use atp::client::Observe;
use axum::extract::ws;
use axum::response::Response;

use crate::RequestContext;
use crate::ws::WebSocket;

const MAX_MESSAGE_SIZE: usize = 2 * 1024 * 1024;
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

#[tracing::instrument(level = "info", name = "agent.connect", parent = ctx.span(), skip(ctx))]
pub async fn connect(ctx: RequestContext, upgrade: ws::WebSocketUpgrade) -> Response {
    upgrade.max_message_size(MAX_MESSAGE_SIZE).on_upgrade(async move |socket| {
        let mut socket = socket.into();
        let observer = AgentObserver {
            ctx: &ctx,
            socket: &socket,
        };

        let req = match session::handshake(&mut socket).await {
            Err(error) => {
                tracing::warn!(?error, "client handshake error");
                return;
            }
            Ok(None) => return,
            Ok(Some(v)) => v,
        };

        observer.on_connect_request(req).await;

        while let Ok(out) = socket.read().await {
            match out {
                atp::Output::Continue => continue,
                atp::Output::Close { code, message } => socket.close_with(code, message.unwrap_or("??".to_string())).await,
                atp::Output::Frame(frame) => observer.on_frame(frame).await,
            };
        }

        ()
    })
}

struct AgentObserver<'a> {
    ctx: &'a RequestContext,
    socket: &'a WebSocket,
}

impl<'a> atp::client::Observe for AgentObserver<'a> {
    fn on_connect_request(
        &self,
        _req: atp::wire::Request<atp::client::ConnectParams>,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), atp::Error>>>> {
        todo!()
    }
}
