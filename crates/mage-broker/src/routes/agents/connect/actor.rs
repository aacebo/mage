use mage_error::Error;

use super::*;

#[tracing::instrument(level = "info", parent = ctx.span(), skip(ctx, socket))]
pub async fn run(ctx: &RequestContext, socket: &mut WebSocket, actor: &mage_types::actors::Actor) -> Result<(), Error> {
    loop {
        match socket.read().await {
            Ok(atp::Output::Continue) => {}
            Ok(atp::Output::Close { .. }) => {
                tracing::debug!("agent requested connection close");
                return Ok(());
            }
            Ok(atp::Output::Frame(atp::wire::Frame::Request(request))) => {
                return Ok(on_request(ctx, socket, actor, request).await?);
            }
            Ok(atp::Output::Frame(_)) => {
                tracing::warn!("agent sent an unsupported ATP frame");
                return Ok(socket
                    .close_with(atp::CloseCode::Policy, "only ATP requests are accepted")
                    .await?);
            }
            Err(error) => {
                tracing::warn!(%error, "agent WebSocket stream failed");
                return Ok(socket.close_with(atp::CloseCode::InternalError, error).await?);
            }
        }
    }
}

#[tracing::instrument(level = "info", parent = ctx.span(), skip(ctx, socket))]
async fn on_request(
    ctx: &RequestContext,
    socket: &mut WebSocket,
    actor: &mage_types::actors::Actor,
    request: atp::wire::Request<atp::client::ClientFrame>,
) -> Result<(), Error> {
    let params = message_params(&request).map_err(mage_error::atp)?;
    let chat = match ctx
        .storage()
        .chats()
        .get_open_for_actor(params.chat_id, actor.tenant_id, actor.id)
        .await
    {
        Ok(Some(chat)) => chat,
        Ok(None) => {
            tracing::warn!(chat_id = %params.chat_id, "agent attempted to send to an unavailable chat");
            return Ok(socket
                .close_with(atp::CloseCode::InvalidData, "chat is unavailable for this agent")
                .await?);
        }
        Err(error) => {
            tracing::error!(%error, chat_id = %params.chat_id, "failed to validate agent chat access");
            let _ = socket.close_with(atp::CloseCode::InternalError, &error).await?;
            return Err(error);
        }
    };

    let (content, metadata) = match convert_message(params) {
        Ok(value) => value,
        Err(error) => {
            return Err(mage_error::internal(error));
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
            let _ = socket
                .close_with(atp::CloseCode::InternalError, "failed to persist message")
                .await;
            return Err(error);
        }
    };

    if let Err(error) = socket.close_with(atp::CloseCode::InternalError, request.id).await {
        tracing::warn!(%error, trace_id = %request.id, chat_id = %chat.id, "failed to acknowledge agent message");
        return Err(error.into());
    }

    tracing::info!(
        trace_id = %request.id,
        chat_id = %chat.id,
        event_id = %event.id,
        "accepted agent message"
    );

    Ok(())
}
