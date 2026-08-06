use std::sync::Arc;

use axum::Router;
use axum::routing::get;

use crate::state;

mod connect;

pub fn router() -> Router<Arc<state::Session>> {
    Router::new().route("/connect", get(connect::connect))
}
