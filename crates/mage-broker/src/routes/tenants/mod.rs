use std::sync::Arc;

use axum::Router;

use crate::state;

pub mod agents;
pub mod logs;
pub mod messages;

pub fn router() -> Router<Arc<state::Session>> {
    Router::new()
        .nest("/agents", agents::router())
        .nest("/logs", logs::router())
        .nest("/messages", messages::router())
}
