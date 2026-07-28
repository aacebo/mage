mod get;

use std::sync::Arc;

use axum::Router;
use axum::routing::get;

use crate::Context;

pub fn router() -> Router<Arc<Context>> {
    Router::new().route("/", get(get::get))
}
