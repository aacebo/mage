mod context;
mod requests;

use atp::Socket;
use axum::extract::ws;
use axum::response::Response;

use crate::state;
use crate::ws::WebSocket;

const MAX_MESSAGE_SIZE: usize = 2 * 1024 * 1024;

pub async fn connect(session: state::http::HttpSession, upgrade: ws::WebSocketUpgrade) -> Response {
    upgrade.max_message_size(MAX_MESSAGE_SIZE).on_upgrade(async move |socket| {
        let mut socket = socket.into();

        match run(session, &mut socket).await {
            Err(err) => {
                let code = match err.name() {
                    "json" | "parse" | "atp_parse" => atp::CloseCode::InvalidData,
                    "atp" | "bad_request" | "unauthorized" => atp::CloseCode::Policy,
                    _ => atp::CloseCode::InternalError,
                };

                let reason = match err.name() {
                    "unauthorized" => err.message(),
                    _ => code.as_str(),
                };

                let _ = socket.close(code, Some(reason)).await;
            }
            Ok(_) => {}
        };

        ()
    })
}

#[tracing::instrument(
    level = "info",
    name = "agent.session",
    parent = session.span(),
    skip(session, socket),
    fields(agent_id = tracing::field::Empty, tenant_id = tracing::field::Empty)
)]
async fn run(session: state::http::HttpSession, socket: &mut WebSocket) -> Result<(), mage_error::Error> {
    tracing::debug!("opening agent connection");
    context::Agent::connect(session, socket).await?.run().await
}
