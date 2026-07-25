use actix_web::{HttpResponse, Result, get, web};

use crate::RequestContext;

#[get("")]
pub async fn get(
    ctx: RequestContext,
    tenant_id: web::Path<uuid::Uuid>,
    query: web::Query<storage::actors::Query>,
) -> Result<HttpResponse> {
    let actors = ctx
        .storage()
        .actors()
        .get(query.into_inner().tenant(tenant_id.into_inner()))
        .await?;
    Ok(HttpResponse::Ok().json(actors))
}
