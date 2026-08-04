use mage_error::Error;

use super::*;

#[tracing::instrument(level = "info", parent = ctx.span(), skip(ctx))]
pub async fn run(ctx: RequestContext, socket: impl Into<WebSocket> + std::fmt::Debug) -> Result<(), Error> {
    tracing::debug!("opening agent connection");
    let mut socket = socket.into();
    let req = match tokio::time::timeout(HANDSHAKE_TIMEOUT, handshake(&mut socket)).await {
        Err(_) => {
            tracing::warn!("agent connection timed out before ATP connect request");
            return Ok(socket.close_with(atp::CloseCode::Policy, "connect request timeout").await?);
        }
        Ok(Ok(Some(request))) => request,
        Ok(Ok(None)) => {
            tracing::debug!("agent closed before ATP connect request");
            return Ok(socket.close().await?);
        }
        Ok(Err(error)) => {
            return Ok(socket.close_with(atp::CloseCode::Policy, error).await?);
        }
    };

    let actor = async {
        let stored_secret = ctx
            .storage()
            .actors()
            .get_secret(req.params.id)
            .await?
            .ok_or(mage_error::unauthorized("unauthorized"))?;

        if stored_secret != req.params.secret {
            return Err(mage_error::unauthorized("unauthorized"));
        }

        let mut actor = ctx
            .storage()
            .actors()
            .get_by_id(req.params.id)
            .await?
            .ok_or(mage_error::unauthorized("unauthorized"))?;

        actor.name = req.params.name.clone();

        if let Some(agent) = &mut actor.agent {
            agent.description = req.params.description.clone();
            agent.skills = req
                .params
                .skills
                .into_iter()
                .map(|s| mage_types::actors::Skill {
                    name: s.name,
                    display_name: s.display_name,
                    description: s.description,
                })
                .collect();
        }

        ctx.storage().actors().update(actor).await?;

        let actor = ctx
            .storage()
            .actors()
            .connect(req.params.id)
            .await?
            .ok_or(mage_error::unauthorized("unauthorized"))?;

        if let Err(error) = ctx.enqueue(actor.tenant_id, "actor.update", actor.clone()).await {
            let _ = ctx.storage().actors().disconnect(actor.id).await;
            return Err(mage_error::internal(error));
        }

        Ok(actor)
    }
    .await?;

    tracing::Span::current().record("agent_id", tracing::field::display(actor.id));
    tracing::Span::current().record("tenant_id", tracing::field::display(actor.tenant_id));

    if let Err(error) = socket.close_with(atp::CloseCode::InternalError, req.id).await {
        tracing::warn!(%error, "failed to acknowledge agent connect request");
        disconnect(&ctx, actor.id).await;
        return Ok(());
    }

    tracing::info!(
        instances = actor.agent.as_ref().map(|agent| agent.instances),
        "agent connected"
    );

    actor::run(&ctx, &mut socket, &actor).await?;
    disconnect(&ctx, actor.id).await;
    Ok(())
}

pub async fn handshake(socket: &mut WebSocket) -> Result<Option<atp::wire::Request<atp::client::ConnectParams>>, Error> {
    loop {
        let output = socket.read().await?;

        match output {
            atp::Output::Continue => continue,
            atp::Output::Close { code, message } => {
                tracing::debug!("agent requested connection close during handshake");
                socket.close_with(code, message.unwrap_or("??".to_string())).await?;
                return Ok(None);
            }
            atp::Output::Frame(atp::wire::Frame::Request(request)) => {
                if let atp::client::ClientFrame::Params(atp::client::ClientParams::Connect(params)) = request.params {
                    return Ok(Some(atp::wire::Request {
                        id: request.id,
                        method: request.method,
                        params,
                    }));
                } else {
                    return Err(mage_error::atp("expected connect request as first frame"));
                }
            }
            atp::Output::Frame(_) => {
                socket
                    .close_with(atp::CloseCode::Policy, "connect request must be the first ATP frame")
                    .await?;
                return Ok(None);
            }
        }
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
