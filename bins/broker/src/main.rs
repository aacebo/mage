use actix_files::Files;
use actix_web::{App, HttpServer, web};
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::EnvFilter;

mod config;
mod context;
mod extract;
mod routes;

pub use config::{Config, ConsoleConfig};
pub use context::*;

#[actix_web::main]
async fn main() -> error::Result<()> {
    tracing_subscriber::fmt()
        .compact()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new("broker=debug")))
        .init();

    let config = Config::from_env()?;
    let pool = PgPoolOptions::new().max_connections(5).connect(&config.database_url).await?;

    sqlx::migrate!("../../crates/storage/migrations")
        .run(&pool)
        .await
        .map_err(error::sql)?;

    let socket = amqp::new(&config.rabbitmq_url)
        .with_app_id("neuron::broker")
        .connect()
        .await?;

    let ctx = Context::new(pool, socket, config.console.clone());
    let console_enabled = config.console.enabled;
    tracing::info!(port = config.port, console_enabled, "starting broker");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(ctx.clone()))
            .wrap(RequestContextMiddleware)
            .service(routes::index::get)
            .service(routes::agents::connect)
            .service(routes::agents::create)
            .service(routes::messages::create)
            .configure(move |services| {
                services.service(routes::logs::scope());

                if console_enabled {
                    routes::console::configure(services);
                }

                let static_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("static");
                services.service(
                    Files::new("/static", static_dir)
                        .index_file("index.html")
                        .show_files_listing(),
                );
            })
    })
    .bind(("0.0.0.0", config.port))?
    .run()
    .await?;

    Ok(())
}
