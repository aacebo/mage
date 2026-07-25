use actix_web::{HttpRequest, HttpResponse, get, rt, web};
use actix_ws::{AggregatedMessage, CloseCode, CloseReason};
use futures_util::StreamExt;
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

#[get("/connect")]
pub async fn connect(
    ctx: RequestContext,
    req: HttpRequest,
    stream: web::Payload,
    actor: extract::Agent,
) -> error::Result<HttpResponse> {
    let (response, session, stream) = actix_ws::handle(&req, stream)?;
    let stream = stream.aggregate_continuations().max_continuation_size(MAX_MESSAGE_SIZE);
    let span = tracing::info_span!(
        parent: ctx.span(),
        "agent.connection",
        agent_id = %actor.id,
        tenant_id = %actor.tenant_id,
    );

    rt::spawn(run_session(ctx, session, stream, actor).instrument(span));
    Ok(response)
}

async fn run_session(
    ctx: RequestContext,
    mut session: actix_ws::Session,
    mut stream: actix_ws::AggregatedMessageStream,
    actor: extract::Agent,
) {
    tracing::debug!("opening agent connection");
    let actor = match ctx.storage().actors().connect(actor.id).await {
        Ok(Some(actor)) => actor,
        Ok(None) => {
            tracing::error!("agent disappeared before connection state could be updated");
            close(session, CloseCode::Error, "failed to update agent connection").await;
            return;
        }
        Err(error) => {
            tracing::error!(%error, "failed to update agent connection state");
            close(session, CloseCode::Error, "failed to update agent connection").await;
            return;
        }
    };

    let connection_event = match ctx.enqueue(actor.tenant_id, "actor.update", actor.clone()).await {
        Ok(event) => event,
        Err(error) => {
            tracing::error!(%error, "failed to enqueue agent connection event");
            let _ = ctx.storage().actors().disconnect(actor.id).await;
            close(session, CloseCode::Error, "failed to persist connection event").await;
            return;
        }
    };

    if emit(&mut session, &connection_event).await.is_err() {
        tracing::warn!("failed to emit agent connection event");
        disconnect(&ctx, actor.id).await;
        return;
    }

    tracing::info!(
        instances = actor.agent.as_ref().map(|agent| agent.instances),
        "agent connected"
    );

    while let Some(message) = stream.next().await {
        match message {
            Ok(AggregatedMessage::Text(text)) => {
                let Ok(command) = serde_json::from_str::<Command>(&text) else {
                    tracing::warn!("closing agent connection after invalid command");
                    close(session.clone(), CloseCode::Invalid, "invalid command").await;
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
                    close(session.clone(), CloseCode::Invalid, "invalid message content").await;
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
                            close(session.clone(), CloseCode::Policy, "chat is unavailable for this agent").await;
                            break;
                        }
                        Err(error) => {
                            tracing::error!(%error, %chat_id, "failed to validate agent chat access");
                            close(session.clone(), CloseCode::Error, "failed to validate chat access").await;
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
                        close(session.clone(), CloseCode::Error, "failed to persist message").await;
                        break;
                    }
                };

                if emit(&mut session, &event).await.is_err() {
                    tracing::warn!(%trace_id, ?chat_id, "failed to return agent message event");
                    break;
                }

                tracing::info!(%trace_id, ?chat_id, event_id = %event.id, "accepted agent message");
            }
            Ok(AggregatedMessage::Ping(bytes)) => {
                if session.pong(&bytes).await.is_err() {
                    tracing::debug!("agent connection closed while sending pong");
                    break;
                }
            }
            Ok(AggregatedMessage::Pong(_)) => {}
            Ok(AggregatedMessage::Close(reason)) => {
                tracing::debug!(?reason, "agent requested connection close");
                break;
            }
            Err(error) => {
                tracing::warn!(%error, "agent WebSocket stream failed");
                break;
            }
            Ok(AggregatedMessage::Binary(_)) => {
                tracing::warn!("closing agent connection after binary command");
                close(session.clone(), CloseCode::Unsupported, "text commands required").await;
                break;
            }
        }
    }

    disconnect(&ctx, actor.id).await;
}

async fn emit(session: &mut actix_ws::Session, event: &types::events::Event) -> error::Result<()> {
    session.text(serde_json::to_string(event)?).await.map_err(error::http)
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

async fn close(session: actix_ws::Session, code: CloseCode, description: &str) {
    let _ = session
        .close(Some(CloseReason {
            code,
            description: Some(description.to_string()),
        }))
        .await;
}

#[cfg(test)]
mod tests {
    use serde_valid::Validate;

    use super::Command;

    #[test]
    fn parses_agent_message_command() {
        let command = serde_json::from_str::<Command>(
            r#"{
                "type":"message_send",
                "trace_id":"00000000-0000-0000-0000-000000000001",
                "content":[{"type":"text","text":"hello"}],
                "metadata":{"source":"test"}
            }"#,
        );
        assert!(matches!(command, Ok(Command::MessageSend { .. })));
    }

    #[test]
    fn rejects_invalid_agent_message_content() {
        let command = serde_json::from_str::<Command>(
            r#"{
                "type":"message_send",
                "content":[],
                "metadata":{}
            }"#,
        )
        .unwrap();
        let Command::MessageSend { content, .. } = command;
        assert!(content.validate().is_err());
    }
}
