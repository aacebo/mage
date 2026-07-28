use std::sync::Arc;

use axum::Router;
use axum::routing::post;

use crate::Context;

mod create;

pub fn router() -> Router<Arc<Context>> {
    Router::new().route("/", post(create::create))
}
