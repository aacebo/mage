use std::sync::Arc;

use axum::Router;
use axum::routing::post;

use crate::state;

mod create;

pub fn router() -> Router<Arc<state::Session>> {
    Router::new().route("/", post(create::create))
}
