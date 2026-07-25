use actix_web::{HttpResponse, post, web};
use error::Result;
use serde_valid::Validate;

use crate::{RequestContext, extract};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Validate)]
struct Request {
    #[serde(default)]
    pub chat_id: Option<uuid::Uuid>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,

    #[validate]
    pub content: types::data::Contents,

    #[serde(default)]
    pub metadata: types::data::Metadata,

    #[validate]
    pub from: FromUser,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Validate)]
struct FromUser {
    pub id: String,
    pub name: String,
}

#[post("")]
pub async fn create(ctx: RequestContext, tenant_id: web::Path<uuid::Uuid>, body: extract::Json<Request>) -> Result<HttpResponse> {
    let body = body.into_inner();
    let tenant_id = tenant_id.into_inner();
    let from = match ctx
        .storage()
        .actors()
        .get_by_external_id(tenant_id, body.from.id.clone())
        .await?
    {
        Some(actor) => actor,
        None => {
            let actor = ctx
                .storage()
                .actors()
                .create(types::actors::Actor {
                    id: uuid::Uuid::now_v7(),
                    external_id: Some(body.from.id.clone()),
                    tenant_id,
                    role: types::actors::Role::User,
                    name: body.from.name,
                    agent: None,
                    metadata: Default::default(),
                    embedding: None,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                })
                .await?;

            ctx.enqueue(actor.tenant_id, "actor.create", actor.clone()).await?;
            actor
        }
    };

    if let Some(chat_id) = body.chat_id {
        let chat = ctx.storage().chats().get_open_for_actor(chat_id, tenant_id, from.id).await?;

        if chat.is_none() {
            return Err(error::bad_request("chat is unavailable for this sender"));
        }
    }

    let message = types::chats::InboundMessage {
        tenant_id,
        chat_id: body.chat_id,
        subject: body.subject,
        content: body.content,
        metadata: body.metadata,
        sent_by: from.into(),
    };

    ctx.enqueue(message.tenant_id, "message.inbound", message.clone()).await?;

    // 1. create/update actor.
    // 2. create embedding of message content ??.
    // 3. search for agents using said embedding.
    // 4. create a new chat with the relevant agents and the from user.
    // 5. on message create, generate message summary/embedding/annotations/artifacts

    Ok(HttpResponse::Ok().json(message))
}
