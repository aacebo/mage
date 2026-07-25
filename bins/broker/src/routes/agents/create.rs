use actix_web::{HttpResponse, post};
use error::Result;
use serde_valid::Validate;

use crate::{RequestContext, extract};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Validate)]
struct Request {
    pub tenant_id: uuid::Uuid,
    pub external_id: Option<String>,
    pub name: String,
    pub description: String,
    #[validate]
    #[serde(default)]
    pub skills: Vec<types::actors::Skill>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct Response<'a> {
    pub secret: &'a str,
    pub actor: &'a types::actors::Actor,
}

#[post("")]
pub async fn create(ctx: RequestContext, body: extract::Json<Request>) -> Result<HttpResponse> {
    let body = body.into_inner();
    let secret = types::secret::new();
    let actor = ctx
        .storage()
        .actors()
        .create(types::actors::Actor {
            id: uuid::Uuid::now_v7(),
            external_id: body.external_id,
            tenant_id: body.tenant_id,
            role: types::actors::Role::Agent,
            name: body.name,
            agent: Some(types::actors::Agent {
                status: types::actors::AgentStatus::Offline,
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

    let res = HttpResponse::Created().json(Response {
        secret: &secret,
        actor: &actor,
    });

    ctx.enqueue(actor.tenant_id, "actor.create", actor).await?;
    Ok(res)
}
