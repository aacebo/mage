use std::sync::Arc;

use axum::Router;
use axum::routing::get;

use crate::state;

mod create;
mod get;

pub fn router() -> Router<Arc<state::Session>> {
    Router::new().route("/", get(get::get).post(create::create))
}
