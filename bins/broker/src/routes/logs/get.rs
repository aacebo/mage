use actix_web::{HttpResponse, Result, get, web};

use crate::RequestContext;

#[get("/logs")]
pub async fn get(
    ctx: RequestContext,
    tenant_id: web::Path<uuid::Uuid>,
    query: web::Query<storage::logs::Query>,
) -> Result<HttpResponse> {
    let logs = ctx
        .storage()
        .logs()
        .get(query.into_inner().tenant(tenant_id.into_inner()))
        .await?;
    Ok(HttpResponse::Ok().json(logs))
}
