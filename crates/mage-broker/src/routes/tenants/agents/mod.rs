use std::sync::Arc;

use axum::Router;
use axum::routing::get;

use crate::Context;

mod create;
mod get;

pub fn router() -> Router<Arc<Context>> {
    Router::new().route("/", get(get::get).post(create::create))
}
