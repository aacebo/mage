use std::time::Duration;

use atp::Socket;
use axum::extract::ws;
use axum::response::Response;
use serde_valid::Validate;
use tracing::Instrument;

use crate::RequestContext;
use crate::ws::{INTERNAL_ERROR_CLOSE, INVALID_DATA_CLOSE, POLICY_CLOSE, WebSocket};

const MAX_MESSAGE_SIZE: usize = 2 * 1024 * 1024;
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
const INVALID_CREDENTIALS: &str = "invalid agent credentials";

pub async fn connect(ctx: RequestContext, upgrade: ws::WebSocketUpgrade) -> Response {
    let span = tracing::info_span!(
        parent: ctx.span(),
        "agent.session",
        agent_id = tracing::field::Empty,
        tenant_id = tracing::field::Empty,
    );

    upgrade
        .max_message_size(MAX_MESSAGE_SIZE)
        .on_upgrade(move |socket| run_session(ctx, socket).instrument(span))
}

async fn run_session(ctx: RequestContext, socket: impl Into<WebSocket>) {
    tracing::debug!("opening agent connection");
    let mut socket = socket.into();

    let (request_id, params) = match tokio::time::timeout(HANDSHAKE_TIMEOUT, read_connect(&mut socket)).await {
        Err(_) => {
            tracing::warn!("agent connection timed out before ATP connect request");
            close(&mut socket, POLICY_CLOSE, "connect request timed out").await;
            return;
        }
        Ok(Ok(Some(request))) => request,
        Ok(Ok(None)) => {
            tracing::debug!("agent closed before ATP connect request");
            return;
        }
        Ok(Err(HandshakeError::Request { id, error })) => {
            let _ = respond_error(&mut socket, id, error).await;
            close(&mut socket, POLICY_CLOSE, "invalid connect request").await;
            return;
        }
        Ok(Err(HandshakeError::Close { code, reason })) => {
            close(&mut socket, code, reason).await;
            return;
        }
    };

    let actor = match establish(&ctx, params).await {
        Ok(actor) => actor,
        Err(EstablishError::Credentials) => {
            tracing::warn!("agent authentication rejected");
            let _ = respond_error(
                &mut socket,
                request_id,
                atp::wire::Error::invalid_request(INVALID_CREDENTIALS),
            )
            .await;
            close(&mut socket, POLICY_CLOSE, INVALID_CREDENTIALS).await;
            return;
        }
        Err(EstablishError::Internal(error)) => {
            tracing::error!(%error, "failed to establish agent session");
            let _ = respond_error(
                &mut socket,
                request_id,
                atp::wire::Error::internal("failed to establish agent session"),
            )
            .await;
            close(&mut socket, INTERNAL_ERROR_CLOSE, "failed to establish agent session").await;
            return;
        }
    };

    tracing::Span::current().record("agent_id", tracing::field::display(actor.id));
    tracing::Span::current().record("tenant_id", tracing::field::display(actor.tenant_id));

    if let Err(error) = respond_ok(&mut socket, request_id).await {
        tracing::warn!(%error, "failed to acknowledge agent connect request");
        disconnect(&ctx, actor.id).await;
        return;
    }

    tracing::info!(
        instances = actor.agent.as_ref().map(|agent| agent.instances),
        "agent connected"
    );

    run_authenticated(&ctx, &mut socket, &actor).await;
    disconnect(&ctx, actor.id).await;
}

async fn read_connect(
    socket: &mut WebSocket,
) -> Result<Option<(uuid::Uuid, atp::client::params::ConnectParams)>, HandshakeError> {
    loop {
        let output = socket.read().await.map_err(read_error)?;

        match output {
            atp::Output::Continue => continue,
            atp::Output::Close { code, message } => {
                tracing::debug!(code, ?message, "agent requested connection close during handshake");
                return Ok(None);
            }
            atp::Output::Frame(atp::wire::Frame::Request(request)) => {
                let params = connect_params(&request).map_err(|error| HandshakeError::Request { id: request.id, error })?;
                return Ok(Some((request.id, params)));
            }
            atp::Output::Frame(_) => {
                return Err(HandshakeError::Close {
                    code: POLICY_CLOSE,
                    reason: "connect request must be the first ATP frame",
                });
            }
        }
    }
}

async fn establish(
    ctx: &RequestContext,
    params: atp::client::params::ConnectParams,
) -> Result<mage_types::actors::Actor, EstablishError> {
    let stored_secret = ctx
        .storage()
        .actors()
        .get_secret(params.id)
        .await
        .map_err(EstablishError::Internal)?
        .ok_or(EstablishError::Credentials)?;

    if stored_secret != params.secret {
        return Err(EstablishError::Credentials);
    }

    let mut actor = ctx
        .storage()
        .actors()
        .get_by_id(params.id)
        .await
        .map_err(EstablishError::Internal)?
        .ok_or(EstablishError::Credentials)?;
    sync_profile(&mut actor, &params).ok_or(EstablishError::Credentials)?;

    ctx.storage().actors().update(actor).await.map_err(EstablishError::Internal)?;

    let actor = ctx
        .storage()
        .actors()
        .connect(params.id)
        .await
        .map_err(EstablishError::Internal)?
        .ok_or(EstablishError::Credentials)?;

    if let Err(error) = ctx.enqueue(actor.tenant_id, "actor.update", actor.clone()).await {
        let _ = ctx.storage().actors().disconnect(actor.id).await;
        return Err(EstablishError::Internal(error));
    }

    Ok(actor)
}

async fn run_authenticated(ctx: &RequestContext, socket: &mut WebSocket, actor: &mage_types::actors::Actor) {
    loop {
        match socket.read().await {
            Ok(atp::Output::Continue) => {}
            Ok(atp::Output::Close { code, message }) => {
                tracing::debug!(code, ?message, "agent requested connection close");
                return;
            }
            Ok(atp::Output::Frame(atp::wire::Frame::Request(request))) => {
                if !handle_request(ctx, socket, actor, request).await {
                    return;
                }
            }
            Ok(atp::Output::Frame(_)) => {
                tracing::warn!("agent sent an unsupported ATP frame");
                close(socket, POLICY_CLOSE, "only ATP requests are accepted").await;
                return;
            }
            Err(error) => {
                tracing::warn!(%error, "agent WebSocket stream failed");
                let (code, reason) = close_for_read_error(&error);
                close(socket, code, reason).await;
                return;
            }
        }
    }
}

async fn handle_request(
    ctx: &RequestContext,
    socket: &mut WebSocket,
    actor: &mage_types::actors::Actor,
    request: atp::wire::Request<atp::client::ClientFrame>,
) -> bool {
    let params = match message_params(&request) {
        Ok(params) => params,
        Err(error) => {
            return respond_error(socket, request.id, error).await.is_ok();
        }
    };

    let chat = match ctx
        .storage()
        .chats()
        .get_open_for_actor(params.chat_id, actor.tenant_id, actor.id)
        .await
    {
        Ok(Some(chat)) => chat,
        Ok(None) => {
            tracing::warn!(chat_id = %params.chat_id, "agent attempted to send to an unavailable chat");
            return respond_error(
                socket,
                request.id,
                atp::wire::Error::invalid_params("chat is unavailable for this agent"),
            )
            .await
            .is_ok();
        }
        Err(error) => {
            tracing::error!(%error, chat_id = %params.chat_id, "failed to validate agent chat access");
            let _ = respond_error(
                socket,
                request.id,
                atp::wire::Error::internal("failed to validate chat access"),
            )
            .await;
            close(socket, INTERNAL_ERROR_CLOSE, "failed to validate chat access").await;
            return false;
        }
    };

    let (content, metadata) = match convert_message(params) {
        Ok(value) => value,
        Err(error) => {
            return respond_error(socket, request.id, atp::wire::Error::invalid_params(error))
                .await
                .is_ok();
        }
    };

    let message = mage_types::chats::InboundMessage {
        tenant_id: actor.tenant_id,
        chat_id: Some(chat.id),
        subject: None,
        content,
        metadata,
        sent_by: actor.clone().into(),
    };

    let event = match ctx
        .enqueue_with_trace(actor.tenant_id, request.id, "message.inbound", message)
        .await
    {
        Ok(event) => event,
        Err(error) => {
            tracing::error!(%error, trace_id = %request.id, chat_id = %chat.id, "failed to enqueue agent message");
            let _ = respond_error(socket, request.id, atp::wire::Error::internal("failed to persist message")).await;
            close(socket, INTERNAL_ERROR_CLOSE, "failed to persist message").await;
            return false;
        }
    };

    if let Err(error) = respond_ok(socket, request.id).await {
        tracing::warn!(%error, trace_id = %request.id, chat_id = %chat.id, "failed to acknowledge agent message");
        return false;
    }

    tracing::info!(
        trace_id = %request.id,
        chat_id = %chat.id,
        event_id = %event.id,
        "accepted agent message"
    );
    true
}

fn convert_skill(skill: atp::types::Skill) -> mage_types::actors::Skill {
    mage_types::actors::Skill {
        name: skill.name,
        display_name: skill.display_name,
        description: skill.description,
    }
}

fn connect_params(
    request: &atp::wire::Request<atp::client::ClientFrame>,
) -> Result<atp::client::params::ConnectParams, atp::wire::Error> {
    if request.method != "connect" {
        return Err(atp::wire::Error::method_not_found(&request.method));
    }

    let params = request
        .params
        .try_params()
        .and_then(atp::client::ClientParams::try_connect)
        .map_err(atp::wire::Error::invalid_params)?
        .clone();
    params.validate().map_err(atp::wire::Error::invalid_params)?;
    Ok(params)
}

fn message_params(
    request: &atp::wire::Request<atp::client::ClientFrame>,
) -> Result<atp::client::params::MessageParams, atp::wire::Error> {
    if request.method != "message" {
        return Err(atp::wire::Error::method_not_found(&request.method));
    }

    let params = request
        .params
        .try_params()
        .and_then(atp::client::ClientParams::try_message)
        .map_err(atp::wire::Error::invalid_params)?
        .clone();
    params.validate().map_err(atp::wire::Error::invalid_params)?;

    if params.reply_to_id.is_some() {
        return Err(atp::wire::Error::invalid_params("reply_to_id is not supported"));
    }

    Ok(params)
}

fn sync_profile(actor: &mut mage_types::actors::Actor, params: &atp::client::params::ConnectParams) -> Option<()> {
    let agent = actor.agent.as_mut()?;
    actor.name.clone_from(&params.name);
    agent.description.clone_from(&params.description);
    agent.skills = params.skills.iter().cloned().map(convert_skill).collect();
    Some(())
}

fn convert_message(
    params: atp::client::params::MessageParams,
) -> Result<(mage_types::data::Contents, mage_types::data::Metadata), String> {
    let mut content = mage_types::data::Contents::default();

    for item in params.content {
        content.push(match item {
            atp::types::Content::Text { text } => mage_types::data::Content::Text { text },
            atp::types::Content::Json { json } => mage_types::data::Content::Json { json },
            atp::types::Content::File { name, file } => mage_types::data::Content::File {
                name,
                file: match file {
                    atp::types::FileContent::Uri { uri } => mage_types::data::FileContent::Uri {
                        uri: uri.parse().map_err(|error| format!("invalid file URI: {error}"))?,
                    },
                    atp::types::FileContent::Base64 { base64 } => mage_types::data::FileContent::Base64 { base64 },
                },
            },
        });
    }

    let mut metadata = mage_types::data::Metadata::default();
    for (key, value) in params.metadata {
        metadata.set(key, value);
    }

    Ok((content, metadata))
}

async fn respond_ok(socket: &mut WebSocket, id: uuid::Uuid) -> Result<(), atp::Error> {
    let response: atp::wire::Response<atp::server::ServerFrame> = atp::wire::Response::Ok { id, result: None };
    socket.write(response).await?;
    socket.flush().await?;
    Ok(())
}

async fn respond_error(socket: &mut WebSocket, id: uuid::Uuid, error: atp::wire::Error) -> Result<(), atp::Error> {
    let response: atp::wire::Response<atp::server::ServerFrame> = atp::wire::Response::Err { id, error };
    socket.write(response).await?;
    socket.flush().await?;
    Ok(())
}

async fn close(socket: &mut WebSocket, code: u16, reason: impl std::fmt::Display) {
    let _ = socket.close_with(code, reason).await;
}

fn read_error(error: atp::Error) -> HandshakeError {
    let (code, reason) = close_for_read_error(&error);
    tracing::warn!(%error, "failed to read ATP connect request");
    HandshakeError::Close { code, reason }
}

fn close_for_read_error(error: &atp::Error) -> (u16, &'static str) {
    match error {
        atp::Error::Json(_) => (INVALID_DATA_CLOSE, "invalid ATP JSON"),
        atp::Error::Protocol(_) => (POLICY_CLOSE, "invalid ATP frame"),
        atp::Error::Socket(_) => (INTERNAL_ERROR_CLOSE, "WebSocket transport failed"),
    }
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

enum HandshakeError {
    Request { id: uuid::Uuid, error: atp::wire::Error },
    Close { code: u16, reason: &'static str },
}

enum EstablishError {
    Credentials,
    Internal(mage_error::Error),
}
