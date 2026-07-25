use actix_web::{HttpResponse, get};
use serde::Serialize;

use crate::RequestContext;

#[derive(Serialize)]
struct IndexResponse {
    start_time: String,
}

#[get("/")]
pub async fn get(ctx: RequestContext) -> HttpResponse {
    HttpResponse::Ok().json(IndexResponse {
        start_time: ctx.start_time().to_rfc3339(),
    })
}
