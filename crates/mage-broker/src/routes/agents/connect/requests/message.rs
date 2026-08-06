use atp::Socket;
use serde_valid::Validate;

use crate::state;
use crate::ws::WebSocket;

pub async fn run(
    session: &state::http::HttpSession,
    socket: &mut WebSocket,
    actor: &mage_types::actors::Actor,
    req: atp::wire::Request<atp::client::MessageParams>,
) -> Result<(), mage_error::Error> {
    if req.method != "message" {
        let response: atp::wire::Response<atp::server::ServerFrame> = atp::wire::Response::Err {
            id: req.id,
            error: atp::error::method_not_found(&req.method),
        };
        socket.write(response).await?;
        return Ok(());
    }

    if let Err(error) = req.params.validate() {
        let response: atp::wire::Response<atp::server::ServerFrame> = atp::wire::Response::Err {
            id: req.id,
            error: atp::error::invalid_params(error),
        };
        socket.write(response).await?;
        return Ok(());
    }

    if req.params.reply_to_id.is_some() {
        let response: atp::wire::Response<atp::server::ServerFrame> = atp::wire::Response::Err {
            id: req.id,
            error: atp::error::invalid_params("reply_to_id is not supported"),
        };
        socket.write(response).await?;
        return Ok(());
    }

    let chat = match session
        .storage()
        .chats()
        .get_open_for_actor(req.params.chat_id, actor.tenant_id, actor.id)
        .await
    {
        Ok(Some(chat)) => chat,
        Ok(None) => {
            tracing::warn!(chat_id = %req.params.chat_id, "agent attempted to send to an unavailable chat");
            let response: atp::wire::Response<atp::server::ServerFrame> = atp::wire::Response::Err {
                id: req.id,
                error: atp::error::invalid_params("chat is unavailable for this agent"),
            };
            socket.write(response).await?;
            return Ok(());
        }
        Err(error) => {
            tracing::error!(%error, chat_id = %req.params.chat_id, "failed to validate agent chat access");
            let response: atp::wire::Response<atp::server::ServerFrame> = atp::wire::Response::Err {
                id: req.id,
                error: atp::error::internal("failed to validate chat access"),
            };
            socket.write(response).await?;
            return Err(error);
        }
    };

    let mut content = mage_types::data::Contents::default();

    for item in req.params.content {
        content.push(match item {
            atp::types::Content::Text { text } => mage_types::data::Content::Text { text },
            atp::types::Content::Json { json } => mage_types::data::Content::Json { json },
            atp::types::Content::File { name, file } => mage_types::data::Content::File {
                name,
                file: match file {
                    atp::types::FileContent::Uri { uri } => {
                        let uri = match uri.parse() {
                            Ok(uri) => uri,
                            Err(error) => {
                                let response: atp::wire::Response<atp::server::ServerFrame> = atp::wire::Response::Err {
                                    id: req.id,
                                    error: atp::error::invalid_params(format!("invalid file URI: {error}")),
                                };
                                socket.write(response).await?;
                                return Ok(());
                            }
                        };
                        mage_types::data::FileContent::Uri { uri }
                    }
                    atp::types::FileContent::Base64 { base64 } => mage_types::data::FileContent::Base64 { base64 },
                },
            },
        });
    }

    let mut metadata = mage_types::data::Metadata::default();

    for (key, value) in req.params.metadata {
        metadata.set(key, value);
    }

    let message = mage_types::chats::InboundMessage {
        tenant_id: actor.tenant_id,
        chat_id: Some(chat.id),
        subject: None,
        content,
        metadata,
        sent_by: actor.clone().into(),
    };

    let event = match session
        .enqueue_with_trace(message.tenant_id, req.id, "message.inbound", message)
        .await
    {
        Ok(event) => event,
        Err(error) => {
            tracing::error!(%error, trace_id = %req.id, chat_id = %chat.id, "failed to enqueue agent message");
            let response: atp::wire::Response<atp::server::ServerFrame> = atp::wire::Response::Err {
                id: req.id,
                error: atp::error::internal("failed to persist message"),
            };
            socket.write(response).await?;
            return Err(error);
        }
    };

    let response: atp::wire::Response<atp::server::ServerFrame> = atp::wire::Response::Ok {
        id: req.id,
        result: None,
    };

    socket.write(response).await?;

    tracing::info!(
        trace_id = %req.id,
        chat_id = %chat.id,
        event_id = %event.id,
        "accepted agent message"
    );

    Ok(())
}
