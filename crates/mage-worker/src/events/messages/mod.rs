use crate::context::EventContext;

mod inbound;

pub async fn run(ctx: &EventContext<'_>) -> mage_error::Result<()> {
    match (ctx.event().key.as_str(), &ctx.event().data) {
        ("message.inbound", mage_types::events::Data::InboundMessage { message }) => inbound::run(ctx, message).await,
        (key, data) => {
            Err(mage_error::bad_request(format!("unsupported event {} => {:#?}", key, data)).trace(ctx.event().trace_id))
        }
    }
}
