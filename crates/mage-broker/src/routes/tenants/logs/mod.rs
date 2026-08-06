mod get;

use std::sync::Arc;

use axum::Router;
use axum::routing::get;

use crate::state;

pub fn router() -> Router<Arc<state::Session>> {
    Router::new().route("/", get(get::get))
}
