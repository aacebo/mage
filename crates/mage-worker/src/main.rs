use sqlx::postgres::PgPoolOptions;
use tracing::Instrument;
use tracing_subscriber::EnvFilter;

mod config;
mod context;
mod events;
mod routing;

pub use config::Config;
pub use context::Context;
pub use routing::*;

use crate::context::EventContext;

#[tokio::main]
async fn main() -> mage_error::Result<()> {
    tracing_subscriber::fmt()
        .compact()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new("mage_worker=debug")))
        .init();

    let config = Config::from_env()?;
    let pool = PgPoolOptions::new().max_connections(5).connect(&config.database_url).await?;
    let socket = mage_amqp::new(&config.rabbitmq_url)
        .with_app_id("mage::worker")
        .with_queue(
            mage_amqp::QueueOptions::new("mage.worker.events")
                .with_binding("actor.*".parse()?)
                .with_binding("message.inbound".parse()?),
        )
        .connect()
        .await?;
    let mut consumer = socket.consume("mage.worker.events").await?;

    tracing::info!("listening...");

    while let Some(result) = consumer.dequeue().await {
        let (delivery, event) = match result {
            Ok(delivery) => delivery,
            Err(error) => {
                tracing::error!(%error, "failed to consume event");
                continue;
            }
        };

        let span = tracing::info_span!(
            "event.delivery",
            event_key = %event.key,
            event_id = %event.id,
            trace_id = %event.trace_id,
        );

        let ctx = Context::new(&pool, span, &socket, config.routing);

        async {
            tracing::debug!("received event delivery");
            let ctx = EventContext::new(&ctx, &delivery, &event);

            if let Err(error) = events::run(&ctx).await {
                tracing::error!(%error, "failed to settle event delivery");
            }
        }
        .instrument(ctx.span().clone())
        .await;
    }

    Ok(())
}
