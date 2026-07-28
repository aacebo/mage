use std::sync::Arc;

use axum::Router;
use axum::routing::get;

use crate::Context;

mod connect;

pub fn router() -> Router<Arc<Context>> {
    Router::new().route("/connect", get(connect::connect))
}
