use futures_util::StreamExt;
use mage_error::Result;

use crate::Socket;

#[derive(Clone)]
pub struct SocketConsumer<'a> {
    socket: &'a Socket,
    consumer: lapin::Consumer,
}

impl<'a> SocketConsumer<'a> {
    pub(crate) fn new(socket: &'a Socket, consumer: lapin::Consumer) -> Self {
        Self { socket, consumer }
    }

    pub fn socket(&self) -> &'a Socket {
        self.socket
    }

    pub async fn cancel(&self) -> Result<()> {
        self.socket
            .channel()
            .basic_cancel(self.consumer.tag().as_str(), lapin::options::BasicCancelOptions::default())
            .await?;

        Ok(())
    }

    pub async fn dequeue(&mut self) -> Option<Result<(lapin::message::Delivery, mage_types::events::Event)>> {
        let delivery = match self.consumer.next().await? {
            Err(err) => return Some(Err(err.into())),
            Ok(v) => v,
        };

        let event = match serde_json::from_slice(&delivery.data) {
            Err(err) => return Some(Err(err.into())),
            Ok(v) => v,
        };

        Some(Ok((delivery, event)))
    }
}
