use atp::Socket;
use atp::client::Observe;

use super::requests;

use crate::{state, ws::WebSocket};

const HANDSHAKE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

pub struct Agent<'a> {
    session: state::http::HttpSession,
    socket: &'a mut WebSocket,
    actor: mage_types::actors::Actor,
}

impl<'a> Agent<'a> {
    pub async fn connect(session: state::http::HttpSession, socket: &'a mut WebSocket) -> Result<Self, mage_error::Error> {
        let first = tokio::time::timeout(HANDSHAKE_TIMEOUT, async {
            loop {
                match socket.read().await {
                    Ok(atp::Output::Continue) => continue,
                    output => return output,
                }
            }
        })
        .await;

        let actor = match first {
            Err(error) => {
                tracing::warn!("agent connection timed out before ATP connect request");
                return Err(mage_error::atp(error));
            }
            Ok(Err(error)) => {
                tracing::warn!(%error, "failed to read ATP connect request");
                return Err(error);
            }
            Ok(Ok(atp::Output::Frame(atp::wire::Frame::<atp::client::ConnectParams>::Request(req)))) => {
                requests::connect::run(&session, socket, req).await?
            }
            _ => {
                return Err(mage_error::atp("connect request must be the first ATP frame"));
            }
        };

        Ok(Self { session, socket, actor })
    }

    pub async fn run(mut self) -> Result<(), mage_error::Error> {
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
        let actor_id = self.actor.id;
        self.session.connections().register(actor_id, sender.downgrade()).await;
        let result = loop {
            tokio::select! {
                output = self.socket.read() => {
                    match output {
                        Ok(atp::Output::Continue) => {}
                        Ok(atp::Output::Close { code, message }) => {
                            tracing::debug!(%code, ?message, "agent requested connection close");
                            break Ok(());
                        }
                        Ok(atp::Output::Frame(frame)) => {
                            if let Err(error) = self.on_frame(frame).await {
                                tracing::warn!(%error, "failed to handle ATP frame");
                                break Err(error);
                            }
                        }
                        Err(error) => {
                            tracing::warn!(%error, "agent WebSocket stream failed");
                            break Err(error);
                        }
                    }
                }
                message = receiver.recv() => {
                    match message {
                        Some(message) => {
                            if let Err(error) = self.socket.send(message).await {
                                tracing::warn!(%error, "failed to send agent WebSocket frame");
                                break Err(mage_error::http(error));
                            }
                        }
                        None => {
                            tracing::debug!("agent connection channel closed");
                            break Ok(());
                        }
                    }
                }
            }
        };

        drop(sender);

        let actor = match self.session.storage().actors().disconnect(actor_id).await {
            Ok(Some(actor)) => actor,
            Ok(None) => {
                tracing::warn!(%actor_id, "agent disappeared before disconnect state could be updated");
                return result;
            }
            Err(error) => {
                tracing::error!(%error, %actor_id, "failed to update agent disconnect state");
                return Err(error);
            }
        };

        let instances = actor.agent.as_ref().map(|agent| agent.instances);

        if let Err(error) = self.session.enqueue(actor.tenant_id, "actor.update", actor).await {
            tracing::error!(%error, %actor_id, "failed to enqueue agent disconnect event");
            return Err(error);
        }

        tracing::info!(%actor_id, ?instances, "agent disconnected");
        result
    }
}

impl<'a> atp::client::Observe for Agent<'a> {
    type Error = mage_error::Error;

    fn on_connect_request(
        &mut self,
        req: atp::wire::Request<atp::client::ConnectParams>,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + '_>> {
        Box::pin(async move {
            let response: atp::wire::Response<atp::server::ServerFrame> = atp::wire::Response::Err {
                id: req.id,
                error: atp::error::invalid_request("agent is already connected"),
            };

            self.socket.write(response).await?;
            Ok(())
        })
    }

    fn on_message_request(
        &mut self,
        req: atp::wire::Request<atp::client::MessageParams>,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + '_>> {
        Box::pin(requests::message::run(&self.session, self.socket, &self.actor, req))
    }
}
