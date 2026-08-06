use axum::extract::{Path, Query};
use axum::response::{IntoResponse, Response};
use mage_error::Result;

use crate::state;

pub async fn get(
    ctx: state::http::HttpSession,
    Path(tenant_id): Path<uuid::Uuid>,
    Query(query): Query<mage_storage::logs::Query>,
) -> Result<Response> {
    let logs = ctx.storage().logs().get(query.tenant(tenant_id)).await?;
    Ok(axum::Json(logs).into_response())
}
