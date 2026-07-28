use axum::extract::{Path, Query};
use axum::response::{IntoResponse, Response};
use error::Result;

use crate::RequestContext;

pub async fn get(
    ctx: RequestContext,
    Path(tenant_id): Path<uuid::Uuid>,
    Query(query): Query<storage::actors::Query>,
) -> Result<Response> {
    let actors = ctx.storage().actors().get(query.tenant(tenant_id)).await?;
    Ok(axum::Json(actors).into_response())
}
