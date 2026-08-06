use axum::extract::{Path, Query};
use axum::response::{IntoResponse, Response};
use mage_error::Result;

use crate::state;

pub async fn get(
    session: state::http::HttpSession,
    Path(tenant_id): Path<uuid::Uuid>,
    Query(query): Query<mage_storage::actors::Query>,
) -> Result<Response> {
    let actors = session.storage().actors().get(query.tenant(tenant_id)).await?;
    Ok(axum::Json(actors).into_response())
}
