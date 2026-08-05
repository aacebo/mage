use std::time::Duration;

use atp::Socket;
use atp::client::Observe;
use axum::extract::ws;
use axum::response::Response;
use serde_valid::Validate;

use crate::RequestContext;
use crate::ws::WebSocket;

const MAX_MESSAGE_SIZE: usize = 2 * 1024 * 1024;
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
const INVALID_CREDENTIALS: &str = "invalid agent credentials";

enum EstablishError {
    Credentials,
    Internal(mage_error::Error),
}

pub async fn connect(ctx: RequestContext, upgrade: ws::WebSocketUpgrade) -> Response {
    upgrade
        .max_message_size(MAX_MESSAGE_SIZE)
        .on_upgrade(move |socket| run(ctx, socket))
}

#[tracing::instrument(
    level = "info",
    name = "agent.session",
    parent = ctx.span(),
    skip(ctx, socket),
    fields(agent_id = tracing::field::Empty, tenant_id = tracing::field::Empty)
)]
async fn run(ctx: RequestContext, socket: ws::WebSocket) {
    tracing::debug!("opening agent connection");

    let mut observer = AgentObserver {
        ctx,
        socket: socket.into(),
        actor: None,
        terminal: false,
    };

    let first = tokio::time::timeout(HANDSHAKE_TIMEOUT, async {
        loop {
            match observer.socket.read().await {
                Ok(atp::Output::Continue) => continue,
                output => return output,
            }
        }
    })
    .await;

    match first {
        Err(_) => {
            tracing::warn!("agent connection timed out before ATP connect request");
            observer.terminal = true;
            let _ = observer
                .socket
                .close(atp::CloseCode::Policy, Some("connect request timeout"))
                .await;
        }
        Ok(Err(error)) => {
            tracing::warn!(%error, "failed to read ATP connect request");
            observer.close_for_error(&error).await;
        }
        Ok(Ok(atp::Output::Close { code, message })) => {
            tracing::debug!(%code, ?message, "agent requested connection close during handshake");
        }
        Ok(Ok(atp::Output::Frame(frame))) => {
            if let Err(error) = observer.on_frame(frame).await {
                tracing::warn!(%error, "invalid ATP connect frame");
                observer.close_for_error(&error).await;
            } else if observer.actor.is_none() && !observer.terminal {
                observer.terminal = true;
                let _ = observer
                    .socket
                    .close(atp::CloseCode::Policy, Some("connect request must be the first ATP frame"))
                    .await;
            }
        }
        Ok(Ok(atp::Output::Continue)) => unreachable!("control frames are skipped by the handshake loop"),
    }

    while observer.actor.is_some() && !observer.terminal {
        match observer.socket.read().await {
            Ok(atp::Output::Continue) => {}
            Ok(atp::Output::Close { code, message }) => {
                tracing::debug!(%code, ?message, "agent requested connection close");
                break;
            }
            Ok(atp::Output::Frame(frame)) => {
                if let Err(error) = observer.on_frame(frame).await {
                    tracing::warn!(%error, "failed to handle ATP frame");
                    observer.close_for_error(&error).await;
                }
            }
            Err(error) => {
                tracing::warn!(%error, "agent WebSocket stream failed");
                observer.close_for_error(&error).await;
            }
        }
    }

    if let Some(connected) = observer.actor.take() {
        let actor_id = connected.id;
        let actor = match observer.ctx.storage().actors().disconnect(actor_id).await {
            Ok(Some(actor)) => actor,
            Ok(None) => {
                tracing::warn!(%actor_id, "agent disappeared before disconnect state could be updated");
                return;
            }
            Err(error) => {
                tracing::error!(%error, %actor_id, "failed to update agent disconnect state");
                return;
            }
        };

        let instances = actor.agent.as_ref().map(|agent| agent.instances);

        if let Err(error) = observer.ctx.enqueue(actor.tenant_id, "actor.update", actor).await {
            tracing::error!(%error, %actor_id, "failed to enqueue agent disconnect event");
            return;
        }

        tracing::info!(%actor_id, ?instances, "agent disconnected");
    }
}

struct AgentObserver {
    ctx: RequestContext,
    socket: WebSocket,
    actor: Option<mage_types::actors::Actor>,
    terminal: bool,
}

impl AgentObserver {
    async fn close_for_error(&mut self, error: &mage_error::Error) {
        let code = match error.name() {
            "json" | "parse" | "atp_parse" => atp::CloseCode::InvalidData,
            "atp" | "bad_request" => atp::CloseCode::Policy,
            _ => atp::CloseCode::InternalError,
        };

        self.terminal = true;
        let _ = self.socket.close(code, Some(code.as_str())).await;
    }
}

impl atp::client::Observe for AgentObserver {
    type Error = mage_error::Error;

    fn on_connect_request(
        &mut self,
        req: atp::wire::Request<atp::client::ConnectParams>,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + '_>> {
        Box::pin(async move {
            if self.actor.is_some() {
                let response: atp::wire::Response<atp::server::ServerFrame> = atp::wire::Response::Err {
                    id: req.id,
                    error: atp::error::invalid_request("agent is already connected"),
                };
                self.socket.write(response).await?;
                return Ok(());
            }

            if req.method != "connect" {
                let response: atp::wire::Response<atp::server::ServerFrame> = atp::wire::Response::Err {
                    id: req.id,
                    error: atp::error::method_not_found(&req.method),
                };
                self.socket.write(response).await?;
                self.terminal = true;
                self.socket
                    .close(atp::CloseCode::Policy, Some("invalid connect request"))
                    .await?;
                return Ok(());
            }

            if let Err(error) = req.params.validate() {
                let response: atp::wire::Response<atp::server::ServerFrame> = atp::wire::Response::Err {
                    id: req.id,
                    error: atp::error::invalid_params(error),
                };
                self.socket.write(response).await?;
                self.terminal = true;
                self.socket
                    .close(atp::CloseCode::Policy, Some("invalid connect request"))
                    .await?;
                return Ok(());
            }

            let actor = async {
                let stored_secret = self
                    .ctx
                    .storage()
                    .actors()
                    .get_secret(req.params.id)
                    .await
                    .map_err(EstablishError::Internal)?
                    .ok_or(EstablishError::Credentials)?;

                if stored_secret != req.params.secret {
                    return Err(EstablishError::Credentials);
                }

                let mut actor = self
                    .ctx
                    .storage()
                    .actors()
                    .get_by_id(req.params.id)
                    .await
                    .map_err(EstablishError::Internal)?
                    .ok_or(EstablishError::Credentials)?;

                let agent = actor.agent.as_mut().ok_or(EstablishError::Credentials)?;
                actor.name.clone_from(&req.params.name);
                agent.description.clone_from(&req.params.description);
                agent.skills = req
                    .params
                    .skills
                    .iter()
                    .cloned()
                    .map(|skill| mage_types::actors::Skill {
                        name: skill.name,
                        display_name: skill.display_name,
                        description: skill.description,
                    })
                    .collect();

                self.ctx
                    .storage()
                    .actors()
                    .update(actor)
                    .await
                    .map_err(EstablishError::Internal)?;

                let actor = self
                    .ctx
                    .storage()
                    .actors()
                    .connect(req.params.id)
                    .await
                    .map_err(EstablishError::Internal)?
                    .ok_or(EstablishError::Credentials)?;

                if let Err(error) = self.ctx.enqueue(actor.tenant_id, "actor.update", actor.clone()).await {
                    let _ = self.ctx.storage().actors().disconnect(actor.id).await;
                    return Err(EstablishError::Internal(error));
                }

                Ok(actor)
            }
            .await;

            let actor = match actor {
                Ok(actor) => actor,
                Err(EstablishError::Credentials) => {
                    tracing::warn!("agent authentication rejected");
                    let response: atp::wire::Response<atp::server::ServerFrame> = atp::wire::Response::Err {
                        id: req.id,
                        error: atp::error::invalid_request(INVALID_CREDENTIALS),
                    };
                    self.socket.write(response).await?;
                    self.terminal = true;
                    self.socket.close(atp::CloseCode::Policy, Some(INVALID_CREDENTIALS)).await?;
                    return Ok(());
                }
                Err(EstablishError::Internal(error)) => {
                    tracing::error!(%error, "failed to establish agent session");
                    let response: atp::wire::Response<atp::server::ServerFrame> = atp::wire::Response::Err {
                        id: req.id,
                        error: atp::error::internal("failed to establish agent session"),
                    };
                    self.socket.write(response).await?;
                    self.terminal = true;
                    self.socket
                        .close(atp::CloseCode::InternalError, Some("failed to establish agent session"))
                        .await?;
                    return Ok(());
                }
            };

            tracing::Span::current().record("agent_id", tracing::field::display(actor.id));
            tracing::Span::current().record("tenant_id", tracing::field::display(actor.tenant_id));
            let instances = actor.agent.as_ref().map(|agent| agent.instances);
            self.actor = Some(actor);

            let response: atp::wire::Response<atp::server::ServerFrame> = atp::wire::Response::Ok {
                id: req.id,
                result: None,
            };
            self.socket.write(response).await?;
            tracing::info!(?instances, "agent connected");
            Ok(())
        })
    }

    fn on_message_request(
        &mut self,
        req: atp::wire::Request<atp::client::MessageParams>,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + '_>> {
        Box::pin(async move {
            let Some(actor) = self.actor.clone() else {
                let response: atp::wire::Response<atp::server::ServerFrame> = atp::wire::Response::Err {
                    id: req.id,
                    error: atp::error::invalid_request("connect request must be the first ATP frame"),
                };
                self.socket.write(response).await?;
                self.terminal = true;
                self.socket
                    .close(atp::CloseCode::Policy, Some("connect request must be the first ATP frame"))
                    .await?;
                return Ok(());
            };

            if req.method != "message" {
                let response: atp::wire::Response<atp::server::ServerFrame> = atp::wire::Response::Err {
                    id: req.id,
                    error: atp::error::method_not_found(&req.method),
                };
                self.socket.write(response).await?;
                return Ok(());
            }

            if let Err(error) = req.params.validate() {
                let response: atp::wire::Response<atp::server::ServerFrame> = atp::wire::Response::Err {
                    id: req.id,
                    error: atp::error::invalid_params(error),
                };
                self.socket.write(response).await?;
                return Ok(());
            }

            if req.params.reply_to_id.is_some() {
                let response: atp::wire::Response<atp::server::ServerFrame> = atp::wire::Response::Err {
                    id: req.id,
                    error: atp::error::invalid_params("reply_to_id is not supported"),
                };
                self.socket.write(response).await?;
                return Ok(());
            }

            let chat = match self
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
                    self.socket.write(response).await?;
                    return Ok(());
                }
                Err(error) => {
                    tracing::error!(%error, chat_id = %req.params.chat_id, "failed to validate agent chat access");
                    let response: atp::wire::Response<atp::server::ServerFrame> = atp::wire::Response::Err {
                        id: req.id,
                        error: atp::error::internal("failed to validate chat access"),
                    };
                    self.socket.write(response).await?;
                    self.terminal = true;
                    self.socket
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
                                        self.socket.write(response).await?;
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

            let event = match self
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
                    self.socket.write(response).await?;
                    self.terminal = true;
                    self.socket
                        .close(atp::CloseCode::InternalError, Some("failed to persist message"))
                        .await?;
                    return Ok(());
                }
            };

            let response: atp::wire::Response<atp::server::ServerFrame> = atp::wire::Response::Ok {
                id: req.id,
                result: None,
            };

            self.socket.write(response).await?;

            tracing::info!(
                trace_id = %req.id,
                chat_id = %chat.id,
                event_id = %event.id,
                "accepted agent message"
            );

            Ok(())
        })
    }
}
