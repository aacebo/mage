use std::sync::Arc;

use axum::{Router, ServiceExt};
use sqlx::postgres::PgPoolOptions;
use tower::Layer;
use tower_http::normalize_path::NormalizePathLayer;
use tower_http::services::ServeDir;
use tracing_subscriber::EnvFilter;

mod config;
mod context;
mod extract;
mod routes;

pub use config::{Config, ConsoleConfig};
pub use context::*;

#[tokio::main]
async fn main() -> mage_error::Result<()> {
    tracing_subscriber::fmt()
        .compact()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new("mage_broker=debug")))
        .init();

    let config = Config::from_env()?;
    let pool = PgPoolOptions::new().max_connections(5).connect(&config.database_url).await?;

    sqlx::migrate!("../mage-storage/migrations")
        .run(&pool)
        .await
        .map_err(mage_error::sql)?;

    let socket = mage_amqp::new(&config.rabbitmq_url)
        .with_app_id("mage::broker")
        .connect()
        .await?;

    let ctx = Arc::new(Context::new(pool, socket, config.console.clone()));
    let console = config.console.enabled;
    tracing::info!(port = config.port, console, "starting broker");

    let mut app = Router::new()
        .route("/health", axum::routing::get(routes::health::get))
        .nest("/agents", routes::agents::router())
        .nest("/tenants/{tenant_id}", routes::tenants::router());

    if console {
        app = app.nest("/console", routes::console::router());
    }

    let static_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("static");
    let app = app
        .nest_service("/static", ServeDir::new(static_dir).append_index_html_on_directories(true))
        .layer(axum::middleware::from_fn_with_state(ctx.clone(), request_middleware))
        .with_state(ctx);
    let app = NormalizePathLayer::trim_trailing_slash().layer(app);
    let app = ServiceExt::<axum::extract::Request>::into_make_service(app);
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", config.port)).await?;

    axum::serve(listener, app).with_graceful_shutdown(on_shutdown()).await?;
    Ok(())
}

async fn on_shutdown() {
    let ctrl_c = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::error!(%error, "failed to listen for Ctrl-C");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => tracing::error!(%error, "failed to listen for SIGTERM"),
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {}
        _ = terminate => {}
    }

    tracing::info!("shutdown signal received");
}
