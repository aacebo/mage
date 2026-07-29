use axum::extract::{FromRequestParts, Query};
use axum::http::request::Parts;

use crate::RequestContext;

const AGENT_ID_HEADER: &str = "X-Agent-Id";
const AGENT_SECRET_HEADER: &str = "X-Agent-Secret";
const INVALID_CREDENTIALS: &str = "invalid agent credentials";

#[derive(Clone, PartialEq, Eq, serde::Deserialize)]
struct Credentials {
    agent_id: uuid::Uuid,
    secret: String,
}

impl Credentials {
    fn from_parts(parts: &Parts) -> mage_error::Result<Self> {
        let agent_id = parts.headers.get(AGENT_ID_HEADER);
        let secret = parts.headers.get(AGENT_SECRET_HEADER);

        match (agent_id, secret) {
            (Some(agent_id), Some(secret)) => {
                let agent_id = agent_id
                    .to_str()
                    .ok()
                    .and_then(|value| value.parse().ok())
                    .ok_or_else(|| mage_error::unauthorized(INVALID_CREDENTIALS));
                let secret = secret
                    .to_str()
                    .map(str::to_owned)
                    .map_err(|_| mage_error::unauthorized(INVALID_CREDENTIALS));

                agent_id.and_then(|agent_id| secret.map(|secret| Self { agent_id, secret }))
            }
            (None, None) => Query::<Self>::try_from_uri(&parts.uri)
                .map(|Query(credentials)| credentials)
                .map_err(|_| mage_error::unauthorized(INVALID_CREDENTIALS)),
            _ => Err(mage_error::unauthorized(INVALID_CREDENTIALS)),
        }
    }
}

#[derive(Debug)]
pub struct Agent(mage_types::actors::Actor);

impl std::ops::Deref for Agent {
    type Target = mage_types::actors::Actor;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Agent {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<S> FromRequestParts<S> for Agent
where
    S: Send + Sync,
{
    type Rejection = mage_error::Error;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let credentials = Credentials::from_parts(parts);
        let ctx = parts
            .extensions
            .get::<RequestContext>()
            .cloned()
            .expect("RequestContext not found in request extensions");

        let Credentials { agent_id, secret } = match credentials {
            Ok(credentials) => credentials,
            Err(error) => {
                tracing::warn!("agent authentication rejected");
                return Err(error);
            }
        };

        let stored_secret = match ctx.storage().actors().get_secret(agent_id).await {
            Ok(Some(secret)) => secret,
            Ok(None) => {
                tracing::warn!(%agent_id, "agent authentication rejected");
                return Err(mage_error::unauthorized(INVALID_CREDENTIALS));
            }
            Err(error) => {
                tracing::error!(%error, %agent_id, "failed to load agent credentials");
                return Err(error);
            }
        };

        if stored_secret != secret {
            tracing::warn!(%agent_id, "agent authentication rejected");
            return Err(mage_error::unauthorized(INVALID_CREDENTIALS));
        }

        let actor = match ctx.storage().actors().get_by_id(agent_id).await {
            Ok(Some(actor)) => actor,
            Ok(None) => {
                tracing::warn!(%agent_id, "agent authentication rejected");
                return Err(mage_error::unauthorized(INVALID_CREDENTIALS));
            }
            Err(error) => {
                tracing::error!(%error, %agent_id, "failed to load authenticated agent");
                return Err(error);
            }
        };

        if actor.agent.is_none() {
            tracing::warn!(%agent_id, "agent authentication rejected");
            return Err(mage_error::unauthorized(INVALID_CREDENTIALS));
        }

        tracing::debug!(%agent_id, tenant_id = %actor.tenant_id, "agent authenticated");
        Ok(Self(actor))
    }
}

#[cfg(test)]
mod tests {
    use axum::http::Request;

    fn parts(uri: &str, agent_id: Option<&str>, secret: Option<&str>) -> axum::http::request::Parts {
        let mut request = Request::builder().uri(uri);

        if let Some(agent_id) = agent_id {
            request = request.header(super::AGENT_ID_HEADER, agent_id);
        }

        if let Some(secret) = secret {
            request = request.header(super::AGENT_SECRET_HEADER, secret);
        }

        request.body(()).unwrap().into_parts().0
    }

    #[test]
    fn complete_headers_take_precedence() {
        let agent_id = uuid::Uuid::now_v7();
        let parts = parts(
            "/agents/connect?agent_id=invalid&secret=query",
            Some(&agent_id.to_string()),
            Some("header"),
        );
        let credentials = super::Credentials::from_parts(&parts).unwrap();
        assert_eq!(credentials.agent_id, agent_id);
        assert_eq!(credentials.secret, "header");
    }

    #[test]
    fn query_credentials_are_the_fallback() {
        let agent_id = uuid::Uuid::now_v7();
        let parts = parts(&format!("/agents/connect?agent_id={agent_id}&secret=query"), None, None);
        let credentials = super::Credentials::from_parts(&parts).unwrap();
        assert_eq!(credentials.agent_id, agent_id);
        assert_eq!(credentials.secret, "query");
    }

    #[test]
    fn partial_headers_are_rejected() {
        let agent_id = uuid::Uuid::now_v7();
        let parts = parts(
            &format!("/agents/connect?agent_id={agent_id}&secret=query"),
            Some(&agent_id.to_string()),
            None,
        );
        let error = match super::Credentials::from_parts(&parts) {
            Ok(_) => panic!("partial headers were accepted"),
            Err(error) => error,
        };
        assert_eq!(error.name(), "unauthorized");
    }
}
