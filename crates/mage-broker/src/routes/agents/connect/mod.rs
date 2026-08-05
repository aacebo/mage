mod requests;

use std::time::Duration;

use atp::Socket;
use atp::client::Observe;
use axum::extract::ws;
use axum::response::Response;
use serde_valid::Validate;

use crate::RequestContext;
use crate::ws::WebSocket;

const MAX_MESSAGE_SIZE: usize = 2 * 1024 * 1024;
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
const INVALID_CREDENTIALS: &str = "invalid agent credentials";

pub async fn connect(ctx: RequestContext, upgrade: ws::WebSocketUpgrade) -> Response {
    upgrade
        .max_message_size(MAX_MESSAGE_SIZE)
        .on_upgrade(move |socket| run(ctx, socket))
}

#[tracing::instrument(
    level = "info",
    name = "agent.session",
    parent = ctx.span(),
    skip(ctx, socket),
    fields(agent_id = tracing::field::Empty, tenant_id = tracing::field::Empty)
)]
async fn run(ctx: RequestContext, socket: ws::WebSocket) {
    tracing::debug!("opening agent connection");

    let mut observer = AgentObserver {
        ctx,
        socket: socket.into(),
        actor: None,
        terminal: false,
    };

    let first = tokio::time::timeout(HANDSHAKE_TIMEOUT, async {
        loop {
            match observer.socket.read().await {
                Ok(atp::Output::Continue) => continue,
                output => return output,
            }
        }
    })
    .await;

    match first {
        Err(_) => {
            tracing::warn!("agent connection timed out before ATP connect request");
            observer.terminal = true;
            let _ = observer
                .socket
                .close(atp::CloseCode::Policy, Some("connect request timeout"))
                .await;
        }
        Ok(Err(error)) => {
            tracing::warn!(%error, "failed to read ATP connect request");
            observer.close_for_error(&error).await;
        }
        Ok(Ok(atp::Output::Close { code, message })) => {
            tracing::debug!(%code, ?message, "agent requested connection close during handshake");
        }
        Ok(Ok(atp::Output::Frame(frame))) => {
            if let Err(error) = observer.on_frame(frame).await {
                tracing::warn!(%error, "invalid ATP connect frame");
                observer.close_for_error(&error).await;
            } else if observer.actor.is_none() && !observer.terminal {
                observer.terminal = true;
                let _ = observer
                    .socket
                    .close(atp::CloseCode::Policy, Some("connect request must be the first ATP frame"))
                    .await;
            }
        }
        Ok(Ok(atp::Output::Continue)) => unreachable!("control frames are skipped by the handshake loop"),
    }

    while observer.actor.is_some() && !observer.terminal {
        match observer.socket.read().await {
            Ok(atp::Output::Continue) => {}
            Ok(atp::Output::Close { code, message }) => {
                tracing::debug!(%code, ?message, "agent requested connection close");
                break;
            }
            Ok(atp::Output::Frame(frame)) => {
                if let Err(error) = observer.on_frame(frame).await {
                    tracing::warn!(%error, "failed to handle ATP frame");
                    observer.close_for_error(&error).await;
                }
            }
            Err(error) => {
                tracing::warn!(%error, "agent WebSocket stream failed");
                observer.close_for_error(&error).await;
            }
        }
    }

    if let Some(connected) = observer.actor.take() {
        let actor_id = connected.id;
        let actor = match observer.ctx.storage().actors().disconnect(actor_id).await {
            Ok(Some(actor)) => actor,
            Ok(None) => {
                tracing::warn!(%actor_id, "agent disappeared before disconnect state could be updated");
                return;
            }
            Err(error) => {
                tracing::error!(%error, %actor_id, "failed to update agent disconnect state");
                return;
            }
        };

        let instances = actor.agent.as_ref().map(|agent| agent.instances);

        if let Err(error) = observer.ctx.enqueue(actor.tenant_id, "actor.update", actor).await {
            tracing::error!(%error, %actor_id, "failed to enqueue agent disconnect event");
            return;
        }

        tracing::info!(%actor_id, ?instances, "agent disconnected");
    }
}

struct AgentObserver {
    ctx: RequestContext,
    socket: WebSocket,
    actor: Option<mage_types::actors::Actor>,
    terminal: bool,
}

impl AgentObserver {
    async fn close_for_error(&mut self, error: &mage_error::Error) {
        let code = match error.name() {
            "json" | "parse" | "atp_parse" => atp::CloseCode::InvalidData,
            "atp" | "bad_request" => atp::CloseCode::Policy,
            _ => atp::CloseCode::InternalError,
        };

        self.terminal = true;
        let _ = self.socket.close(code, Some(code.as_str())).await;
    }
}

impl atp::client::Observe for AgentObserver {
    type Error = mage_error::Error;

    fn on_connect_request(
        &mut self,
        req: atp::wire::Request<atp::client::ConnectParams>,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + '_>> {
        requests::connect::run(self, req)
    }

    fn on_message_request(
        &mut self,
        req: atp::wire::Request<atp::client::MessageParams>,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + '_>> {
        requests::message::run(self, req)
    }
}
