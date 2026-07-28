use std::sync::Arc;

use axum::Router;

use crate::Context;

pub mod agents;
pub mod logs;
pub mod messages;

pub fn router() -> Router<Arc<Context>> {
    Router::new()
        .nest("/agents", agents::router())
        .nest("/logs", logs::router())
        .nest("/messages", messages::router())
}
