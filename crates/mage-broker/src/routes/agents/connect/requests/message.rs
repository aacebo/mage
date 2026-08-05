use super::super::*;

pub fn run(
    state: &mut AgentObserver,
    req: atp::wire::Request<atp::client::MessageParams>,
) -> std::pin::Pin<Box<dyn Future<Output = Result<(), mage_error::Error>> + Send + '_>> {
    Box::pin(async move {
        let Some(actor) = state.actor.clone() else {
            let response: atp::wire::Response<atp::server::ServerFrame> = atp::wire::Response::Err {
                id: req.id,
                error: atp::error::invalid_request("connect request must be the first ATP frame"),
            };

            state.socket.write(response).await?;
            state.terminal = true;
            state
                .socket
                .close(atp::CloseCode::Policy, Some("connect request must be the first ATP frame"))
                .await?;

            return Ok(());
        };

        if req.method != "message" {
            let response: atp::wire::Response<atp::server::ServerFrame> = atp::wire::Response::Err {
                id: req.id,
                error: atp::error::method_not_found(&req.method),
            };
            state.socket.write(response).await?;
            return Ok(());
        }

        if let Err(error) = req.params.validate() {
            let response: atp::wire::Response<atp::server::ServerFrame> = atp::wire::Response::Err {
                id: req.id,
                error: atp::error::invalid_params(error),
            };
            state.socket.write(response).await?;
            return Ok(());
        }

        if req.params.reply_to_id.is_some() {
            let response: atp::wire::Response<atp::server::ServerFrame> = atp::wire::Response::Err {
                id: req.id,
                error: atp::error::invalid_params("reply_to_id is not supported"),
            };
            state.socket.write(response).await?;
            return Ok(());
        }

        let chat = match state
            .ctx
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
                state.socket.write(response).await?;
                return Ok(());
            }
            Err(error) => {
                tracing::error!(%error, chat_id = %req.params.chat_id, "failed to validate agent chat access");
                let response: atp::wire::Response<atp::server::ServerFrame> = atp::wire::Response::Err {
                    id: req.id,
                    error: atp::error::internal("failed to validate chat access"),
                };
                state.socket.write(response).await?;
                state.terminal = true;
                state
                    .socket
                    .close(atp::CloseCode::InternalError, Some("failed to validate chat access"))
                    .await?;
                return Ok(());
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
                                    state.socket.write(response).await?;
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
            sent_by: actor.into(),
        };

        let event = match state
            .ctx
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
                state.socket.write(response).await?;
                state.terminal = true;
                state
                    .socket
                    .close(atp::CloseCode::InternalError, Some("failed to persist message"))
                    .await?;
                return Ok(());
            }
        };

        let response: atp::wire::Response<atp::server::ServerFrame> = atp::wire::Response::Ok {
            id: req.id,
            result: None,
        };

        state.socket.write(response).await?;

        tracing::info!(
            trace_id = %req.id,
            chat_id = %chat.id,
            event_id = %event.id,
            "accepted agent message"
        );

        Ok(())
    })
}
