use atp::Socket;
use serde_valid::Validate;

use crate::state;
use crate::ws::WebSocket;

const INVALID_CREDENTIALS: &str = "invalid agent credentials";

pub async fn run<'a>(
    session: &'a state::http::HttpSession,
    socket: &'a mut WebSocket,
    req: atp::wire::Request<atp::client::ConnectParams>,
) -> Result<mage_types::actors::Actor, mage_error::Error> {
    if req.method != "connect" {
        let response: atp::wire::Response<atp::server::ServerFrame> = atp::wire::Response::Err {
            id: req.id,
            error: atp::error::method_not_found(&req.method),
        };

        socket.write(response).await?;
        return Err(mage_error::atp("invalid connect request"));
    }

    if let Err(error) = req.params.validate() {
        let response: atp::wire::Response<atp::server::ServerFrame> = atp::wire::Response::Err {
            id: req.id,
            error: atp::error::invalid_params(error),
        };

        socket.write(response).await?;
        return Err(mage_error::atp("invalid connect request"));
    }

    let actor = async {
        let stored_secret = session
            .storage()
            .actors()
            .get_secret(req.params.id)
            .await?
            .ok_or_else(|| mage_error::unauthorized(INVALID_CREDENTIALS))?;

        if stored_secret != req.params.secret {
            return Err(mage_error::unauthorized(INVALID_CREDENTIALS));
        }

        let mut actor = session
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

        session.storage().actors().update(actor).await?;

        let actor = session
            .storage()
            .actors()
            .connect(req.params.id)
            .await?
            .ok_or_else(|| mage_error::unauthorized(INVALID_CREDENTIALS))?;

        if let Err(error) = session.enqueue(actor.tenant_id, "actor.update", actor.clone()).await {
            let _ = session.storage().actors().disconnect(actor.id).await;
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
            socket.write(response).await?;
            return Err(error);
        }
        Err(error) => {
            tracing::error!(%error, "failed to establish agent session");
            let response: atp::wire::Response<atp::server::ServerFrame> = atp::wire::Response::Err {
                id: req.id,
                error: atp::error::internal("failed to establish agent session"),
            };
            socket.write(response).await?;
            return Err(error);
        }
    };

    tracing::Span::current().record("agent_id", tracing::field::display(actor.id));
    tracing::Span::current().record("tenant_id", tracing::field::display(actor.tenant_id));

    let instances = actor.agent.as_ref().map(|agent| agent.instances);
    let response: atp::wire::Response<atp::server::ServerFrame> = atp::wire::Response::Ok {
        id: req.id,
        result: None,
    };

    if let Err(error) = socket.write(response).await {
        let _ = session.storage().actors().disconnect(actor.id).await;
        return Err(error);
    }

    tracing::info!(?instances, "agent connected");
    Ok(actor)
}
