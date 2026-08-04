use std::collections::HashSet;
use std::time::Duration;

use axum::body::Bytes;
use axum::extract::Query;
use axum::extract::ws::{CloseCode, CloseFrame, Message, WebSocket, WebSocketUpgrade, close_code};
use axum::response::Response;

use crate::RequestContext;

const REPLAY_BATCH_SIZE: usize = 100;
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(20);

#[derive(Debug, serde::Deserialize)]
pub(super) struct ReplayQuery {
    after_id: Option<uuid::Uuid>,
}

pub async fn connect(ctx: RequestContext, Query(query): Query<ReplayQuery>, upgrade: WebSocketUpgrade) -> Response {
    let tenant_id = ctx.console().tenant_id.unwrap();
    let cursor = query.after_id;

    upgrade
        .max_message_size(64 * 1024)
        .on_upgrade(move |socket| run_stream(ctx, tenant_id, cursor, socket))
}

#[tracing::instrument(
    level = "info",
    name = "console.connection",
    parent = ctx.span(),
    skip(ctx, socket),
    fields(tenant_id = %tenant_id, replay_after_id = ?cursor)
)]
async fn run_stream(ctx: RequestContext, tenant_id: uuid::Uuid, cursor: Option<uuid::Uuid>, mut socket: WebSocket) {
    let binding = "#".parse().expect("the console event binding is valid");
    let mut events = match ctx.socket().subscribe(&[binding]).await {
        Ok(events) => events,
        Err(error) => {
            tracing::error!(%error, "failed to create console AMQP subscription");
            close(&mut socket, close_code::ERROR, "event subscription failed").await;
            return;
        }
    };

    tracing::debug!("created exclusive console AMQP subscription");
    run_event_stream(&ctx, tenant_id, cursor, socket, &mut events).await;

    if let Err(error) = events.cancel().await {
        tracing::warn!(%error, "failed to cancel console AMQP subscription");
    } else {
        tracing::debug!("cancelled console AMQP subscription");
    }
}

async fn run_event_stream(
    ctx: &RequestContext,
    tenant_id: uuid::Uuid,
    mut cursor: Option<uuid::Uuid>,
    mut socket: WebSocket,
    events: &mut mage_amqp::SocketConsumer<'_>,
) {
    let mut sent = HashSet::new();
    let mut replayed = 0_usize;
    tracing::debug!("starting console event replay");

    loop {
        let query = mage_storage::events::query::new()
            .tenant(tenant_id)
            .limit(REPLAY_BATCH_SIZE)
            .ascending();
        let query = match cursor {
            Some(cursor) => query.cursor(cursor),
            None => query,
        };
        let result = match ctx.storage().events().get(query).await {
            Ok(result) => result,
            Err(error) => {
                tracing::error!(%error, "failed to replay console events");
                close(&mut socket, close_code::ERROR, "event replay failed").await;
                return;
            }
        };

        let count = result.items.len();

        for event in result.items {
            cursor = Some(event.id);
            sent.insert(event.id);

            if let Err(error) = emit(&mut socket, &event).await {
                tracing::debug!(%error, event_id = %event.id, "console disconnected during event replay");
                return;
            }

            replayed += 1;
        }

        if count < REPLAY_BATCH_SIZE {
            break;
        }
    }

    tracing::info!(replayed, "console event stream connected");
    let mut heartbeat = tokio::time::interval(HEARTBEAT_INTERVAL);
    heartbeat.tick().await;

    loop {
        tokio::select! {
            message = socket.recv() => {
                match message {
                    Some(Ok(Message::Ping(_) | Message::Pong(_))) => {}
                    Some(Ok(Message::Close(reason))) => {
                        tracing::debug!(?reason, "console requested connection close");
                        return;
                    }
                    Some(Err(error)) => {
                        tracing::warn!(%error, "console WebSocket stream failed");
                        return;
                    }
                    None => {
                        tracing::debug!("console WebSocket stream ended");
                        return;
                    }
                    Some(Ok(Message::Text(_) | Message::Binary(_))) => {
                        tracing::warn!("console attempted to write to its read-only event stream");
                        close(&mut socket, close_code::POLICY, "console stream is read only").await;
                        return;
                    }
                }
            }
            notification = events.dequeue() => {
                let (delivery, event) = match notification {
                    Some(Ok(delivery)) => delivery,
                    Some(Err(error)) => {
                        tracing::error!(%error, "failed to consume console AMQP event");
                        close(&mut socket, close_code::ERROR, "event subscription failed").await;
                        return;
                    }
                    None => {
                        tracing::warn!("console AMQP subscription ended");
                        close(&mut socket, close_code::AGAIN, "event subscription ended").await;
                        return;
                    }
                };

                if event.tenant_id == tenant_id && sent.insert(event.id) {
                    if let Err(error) = emit(&mut socket, &event).await {
                        tracing::debug!(%error, event_id = %event.id, trace_id = %event.trace_id, "console disconnected during live event");
                        return;
                    }

                    tracing::debug!(
                        event_key = %event.key,
                        event_id = %event.id,
                        trace_id = %event.trace_id,
                        "emitted live console event"
                    );
                }

                if let Err(error) = delivery
                    .ack(mage_amqp::lapin::options::BasicAckOptions::default())
                    .await
                {
                    tracing::error!(
                        %error,
                        event_key = %event.key,
                        event_id = %event.id,
                        trace_id = %event.trace_id,
                        "failed to acknowledge console AMQP event"
                    );
                    close(&mut socket, close_code::ERROR, "event acknowledgement failed").await;
                    return;
                }
            }
            _ = heartbeat.tick() => {
                if socket.send(Message::Ping(Bytes::from_static(b"atp"))).await.is_err() {
                    tracing::debug!("console disconnected while sending heartbeat");
                    return;
                }
            }
        }
    }
}

async fn emit(socket: &mut WebSocket, event: &mage_types::events::Event) -> mage_error::Result<()> {
    socket
        .send(Message::Text(serde_json::to_string(event)?.into()))
        .await
        .map_err(mage_error::http)
}

async fn close(socket: &mut WebSocket, code: CloseCode, description: &str) {
    let _ = socket
        .send(Message::Close(Some(CloseFrame {
            code,
            reason: description.to_string().into(),
        })))
        .await;
}
