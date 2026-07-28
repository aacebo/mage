mod connect;
mod index;

use std::sync::Arc;

use axum::Router;
use axum::routing::get;

use crate::Context;

pub fn router() -> Router<Arc<Context>> {
    Router::new()
        .route("/", get(index::page))
        .route("/connect", get(connect::connect))
}
