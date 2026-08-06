use axum::Json;
use serde::Serialize;

use crate::state;

#[derive(Serialize)]
pub(crate) struct IndexResponse {
    started_at: String,
}

pub async fn get(ctx: state::http::HttpSession) -> Json<IndexResponse> {
    Json(IndexResponse {
        started_at: ctx.started_at().to_rfc3339(),
    })
}
