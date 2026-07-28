use axum::extract::ws::{CloseCode, CloseFrame, Message, WebSocket, WebSocketUpgrade, close_code};
use axum::response::Response;
use serde_valid::Validate;
use tracing::Instrument;

use crate::{RequestContext, extract};

const MAX_MESSAGE_SIZE: usize = 2 * 1024 * 1024;

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Command {
    MessageSend {
        #[serde(default)]
        trace_id: Option<uuid::Uuid>,

        #[serde(default)]
        chat_id: Option<uuid::Uuid>,

        #[serde(default)]
        subject: Option<String>,

        content: types::data::Contents,

        #[serde(default)]
        metadata: types::data::Metadata,
    },
}

pub async fn connect(ctx: RequestContext, actor: extract::Agent, upgrade: WebSocketUpgrade) -> Response {
    let span = tracing::info_span!(
        parent: ctx.span(),
        "agent.connection",
        agent_id = %actor.id,
        tenant_id = %actor.tenant_id,
    );

    upgrade
        .max_message_size(MAX_MESSAGE_SIZE)
        .on_upgrade(move |socket| run_session(ctx, socket, actor).instrument(span))
}

async fn run_session(ctx: RequestContext, mut socket: WebSocket, actor: extract::Agent) {
    tracing::debug!("opening agent connection");
    let actor = match ctx.storage().actors().connect(actor.id).await {
        Ok(Some(actor)) => actor,
        Ok(None) => {
            tracing::error!("agent disappeared before connection state could be updated");
            close(&mut socket, close_code::ERROR, "failed to update agent connection").await;
            return;
        }
        Err(error) => {
            tracing::error!(%error, "failed to update agent connection state");
            close(&mut socket, close_code::ERROR, "failed to update agent connection").await;
            return;
        }
    };

    let connection_event = match ctx.enqueue(actor.tenant_id, "actor.update", actor.clone()).await {
        Ok(event) => event,
        Err(error) => {
            tracing::error!(%error, "failed to enqueue agent connection event");
            let _ = ctx.storage().actors().disconnect(actor.id).await;
            close(&mut socket, close_code::ERROR, "failed to persist connection event").await;
            return;
        }
    };

    if emit(&mut socket, &connection_event).await.is_err() {
        tracing::warn!("failed to emit agent connection event");
        disconnect(&ctx, actor.id).await;
        return;
    }

    tracing::info!(
        instances = actor.agent.as_ref().map(|agent| agent.instances),
        "agent connected"
    );

    while let Some(message) = socket.recv().await {
        match message {
            Ok(Message::Text(text)) => {
                let Ok(command) = serde_json::from_str::<Command>(text.as_str()) else {
                    tracing::warn!("closing agent connection after invalid command");
                    close(&mut socket, close_code::INVALID, "invalid command").await;
                    break;
                };

                let Command::MessageSend {
                    trace_id,
                    chat_id,
                    subject,
                    content,
                    metadata,
                } = command;

                if let Err(error) = content.validate() {
                    tracing::warn!(%error, "closing agent connection after invalid message content");
                    close(&mut socket, close_code::INVALID, "invalid message content").await;
                    break;
                }

                if let Some(chat_id) = chat_id {
                    match ctx
                        .storage()
                        .chats()
                        .get_open_for_actor(chat_id, actor.tenant_id, actor.id)
                        .await
                    {
                        Ok(Some(_)) => {}
                        Ok(None) => {
                            tracing::warn!(%chat_id, "agent attempted to send to an unavailable chat");
                            close(&mut socket, close_code::POLICY, "chat is unavailable for this agent").await;
                            break;
                        }
                        Err(error) => {
                            tracing::error!(%error, %chat_id, "failed to validate agent chat access");
                            close(&mut socket, close_code::ERROR, "failed to validate chat access").await;
                            break;
                        }
                    }
                }

                let trace_id = trace_id.unwrap_or_else(uuid::Uuid::now_v7);
                tracing::debug!(%trace_id, ?chat_id, "received agent message command");
                let message = types::chats::InboundMessage {
                    tenant_id: actor.tenant_id,
                    chat_id,
                    subject,
                    content,
                    metadata,
                    sent_by: actor.clone().into(),
                };

                let event = match ctx
                    .enqueue_with_trace(actor.tenant_id, trace_id, "message.inbound", message)
                    .await
                {
                    Ok(event) => event,
                    Err(error) => {
                        tracing::error!(%error, %trace_id, ?chat_id, "failed to enqueue agent message");
                        close(&mut socket, close_code::ERROR, "failed to persist message").await;
                        break;
                    }
                };

                if emit(&mut socket, &event).await.is_err() {
                    tracing::warn!(%trace_id, ?chat_id, "failed to return agent message event");
                    break;
                }

                tracing::info!(%trace_id, ?chat_id, event_id = %event.id, "accepted agent message");
            }
            Ok(Message::Ping(_) | Message::Pong(_)) => {}
            Ok(Message::Close(reason)) => {
                tracing::debug!(?reason, "agent requested connection close");
                break;
            }
            Err(error) => {
                tracing::warn!(%error, "agent WebSocket stream failed");
                break;
            }
            Ok(Message::Binary(_)) => {
                tracing::warn!("closing agent connection after binary command");
                close(&mut socket, close_code::UNSUPPORTED, "text commands required").await;
                break;
            }
        }
    }

    disconnect(&ctx, actor.id).await;
}

async fn emit(socket: &mut WebSocket, event: &types::events::Event) -> error::Result<()> {
    socket
        .send(Message::Text(serde_json::to_string(event)?.into()))
        .await
        .map_err(error::http)
}

async fn disconnect(ctx: &RequestContext, actor_id: uuid::Uuid) {
    let actor = match ctx.storage().actors().disconnect(actor_id).await {
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

    if let Err(error) = ctx.enqueue(actor.tenant_id, "actor.update", actor).await {
        tracing::error!(%error, %actor_id, "failed to enqueue agent disconnect event");
        return;
    }

    tracing::info!(%actor_id, ?instances, "agent disconnected");
}

async fn close(socket: &mut WebSocket, code: CloseCode, description: &str) {
    let _ = socket
        .send(Message::Close(Some(CloseFrame {
            code,
            reason: description.to_string().into(),
        })))
        .await;
}
