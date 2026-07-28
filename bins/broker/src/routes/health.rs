use axum::Json;
use serde::Serialize;

use crate::RequestContext;

#[derive(Serialize)]
pub(crate) struct IndexResponse {
    start_time: String,
}

pub async fn get(ctx: RequestContext) -> Json<IndexResponse> {
    Json(IndexResponse {
        start_time: ctx.start_time().to_rfc3339(),
    })
}
