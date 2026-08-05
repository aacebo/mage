use serde_valid::Validate;

use super::super::*;

pub fn run(
    state: &mut AgentObserver,
    req: atp::wire::Request<atp::client::ConnectParams>,
) -> std::pin::Pin<Box<dyn Future<Output = Result<(), mage_error::Error>> + Send + '_>> {
    Box::pin(async move {
        if state.actor.is_some() {
            let response: atp::wire::Response<atp::server::ServerFrame> = atp::wire::Response::Err {
                id: req.id,
                error: atp::error::invalid_request("agent is already connected"),
            };
            state.socket.write(response).await?;
            return Ok(());
        }

        if req.method != "connect" {
            let response: atp::wire::Response<atp::server::ServerFrame> = atp::wire::Response::Err {
                id: req.id,
                error: atp::error::method_not_found(&req.method),
            };
            state.socket.write(response).await?;
            state.terminal = true;
            state
                .socket
                .close(atp::CloseCode::Policy, Some("invalid connect request"))
                .await?;
            return Ok(());
        }

        if let Err(error) = req.params.validate() {
            let response: atp::wire::Response<atp::server::ServerFrame> = atp::wire::Response::Err {
                id: req.id,
                error: atp::error::invalid_params(error),
            };
            state.socket.write(response).await?;
            state.terminal = true;
            state
                .socket
                .close(atp::CloseCode::Policy, Some("invalid connect request"))
                .await?;
            return Ok(());
        }

        let actor = async {
            let stored_secret = state
                .ctx
                .storage()
                .actors()
                .get_secret(req.params.id)
                .await?
                .ok_or_else(|| mage_error::unauthorized(INVALID_CREDENTIALS))?;

            if stored_secret != req.params.secret {
                return Err(mage_error::unauthorized(INVALID_CREDENTIALS));
            }

            let mut actor = state
                .ctx
                .storage()
                .actors()
                .get_by_id(req.params.id)
                .await?
                .ok_or_else(|| mage_error::unauthorized(INVALID_CREDENTIALS))?;

            let agent = actor
                .agent
                .as_mut()
                .ok_or_else(|| mage_error::unauthorized(INVALID_CREDENTIALS))?;

            actor.name.clone_from(&req.params.name);
            agent.description.clone_from(&req.params.description);
            agent.skills = req
                .params
                .skills
                .iter()
                .cloned()
                .map(|skill| mage_types::actors::Skill {
                    name: skill.name,
                    display_name: skill.display_name,
                    description: skill.description,
                })
                .collect();

            state.ctx.storage().actors().update(actor).await?;

            let actor = state
                .ctx
                .storage()
                .actors()
                .connect(req.params.id)
                .await?
                .ok_or_else(|| mage_error::unauthorized(INVALID_CREDENTIALS))?;

            if let Err(error) = state.ctx.enqueue(actor.tenant_id, "actor.update", actor.clone()).await {
                let _ = state.ctx.storage().actors().disconnect(actor.id).await;
                return Err(mage_error::internal(error));
            }

            Ok(actor)
        }
        .await;

        let actor = match actor {
            Ok(actor) => actor,
            Err(error) if error.name() == "unauthorized" => {
                tracing::warn!("agent authentication rejected");
                let response: atp::wire::Response<atp::server::ServerFrame> = atp::wire::Response::Err {
                    id: req.id,
                    error: atp::error::invalid_request(INVALID_CREDENTIALS),
                };
                state.socket.write(response).await?;
                state.terminal = true;
                state.socket.close(atp::CloseCode::Policy, Some(INVALID_CREDENTIALS)).await?;
                return Ok(());
            }
            Err(error) => {
                tracing::error!(%error, "failed to establish agent session");
                let response: atp::wire::Response<atp::server::ServerFrame> = atp::wire::Response::Err {
                    id: req.id,
                    error: atp::error::internal("failed to establish agent session"),
                };
                state.socket.write(response).await?;
                state.terminal = true;
                state
                    .socket
                    .close(atp::CloseCode::InternalError, Some("failed to establish agent session"))
                    .await?;
                return Ok(());
            }
        };

        tracing::Span::current().record("agent_id", tracing::field::display(actor.id));
        tracing::Span::current().record("tenant_id", tracing::field::display(actor.tenant_id));
        let instances = actor.agent.as_ref().map(|agent| agent.instances);
        state.actor = Some(actor);

        let response: atp::wire::Response<atp::server::ServerFrame> = atp::wire::Response::Ok {
            id: req.id,
            result: None,
        };

        state.socket.write(response).await?;
        tracing::info!(?instances, "agent connected");
        Ok(())
    })
}
