use axum::Json;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response as HttpResponse};
use mage_error::Result;
use serde_valid::Validate;

use crate::{RequestContext, extract};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Validate)]
pub(super) struct Request {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,

    pub name: String,
    pub description: String,

    #[validate]
    #[serde(default)]
    pub skills: Vec<mage_types::actors::Skill>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct Response<'a> {
    pub secret: &'a str,
    pub actor: &'a mage_types::actors::Actor,
}

pub async fn create(
    ctx: RequestContext,
    Path(tenant_id): Path<uuid::Uuid>,
    body: extract::Json<Request>,
) -> Result<HttpResponse> {
    let body = body.into_inner();
    let secret = mage_types::secret::new();
    let actor = ctx
        .storage()
        .actors()
        .create(mage_types::actors::Actor {
            id: uuid::Uuid::now_v7(),
            external_id: body.external_id,
            tenant_id,
            role: mage_types::actors::Role::Agent,
            name: body.name,
            agent: Some(mage_types::actors::Agent {
                status: mage_types::actors::AgentStatus::Offline,
                description: body.description,
                secret: secret.clone(),
                instances: 0,
                skills: body.skills,
            }),
            metadata: Default::default(),
            embedding: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
        .await?;

    let res = (
        StatusCode::CREATED,
        Json(Response {
            secret: &secret,
            actor: &actor,
        }),
    )
        .into_response();

    ctx.enqueue(actor.tenant_id, "actor.create", actor).await?;
    Ok(res)
}
