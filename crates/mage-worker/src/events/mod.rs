use crate::context::EventContext;

pub mod actors;
pub mod messages;

#[tracing::instrument(
    level = "info",
    name = "event.delivery",
    skip(ctx),
    fields(
        event_key = %ctx.event().key,
        event_id = %ctx.event().id,
        trace_id = %ctx.event().trace_id,
    )
)]
pub async fn run(ctx: &EventContext<'_>) -> mage_error::Result<()> {
    tracing::debug!("received event delivery");

    let result = match (ctx.event().key.as_str(), &ctx.event().data) {
        ("actor.create" | "actor.update", mage_types::events::Data::Actor { actor }) => actors::run(ctx, actor.id).await,
        ("message.inbound", mage_types::events::Data::InboundMessage { message: _ }) => messages::run(ctx).await,
        (key, data) => {
            tracing::info!(key, ?data, "unsupported event");
            ctx.reject().await?;
            Ok(())
        }
    };

    if let Err(error) = &result {
        tracing::error!(%error, "failed to settle event delivery");
    }

    result
}
