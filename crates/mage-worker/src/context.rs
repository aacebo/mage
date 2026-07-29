#![allow(unused)]

use chrono::{DateTime, Utc};
use mage_storage::Storage;
use sqlx::PgPool;

#[derive(Clone)]
pub struct Context<'a> {
    pool: &'a PgPool,
    span: tracing::Span,
    socket: &'a mage_amqp::Socket,
    start_time: DateTime<Utc>,
    routing: crate::RoutingPolicy,
}

impl<'a> Context<'a> {
    pub fn new(pool: &'a PgPool, span: tracing::Span, socket: &'a mage_amqp::Socket, routing: crate::RoutingPolicy) -> Self {
        Self {
            pool,
            span,
            socket,
            start_time: Utc::now(),
            routing,
        }
    }

    pub fn span(&self) -> &tracing::Span {
        &self.span
    }

    pub fn start_time(&self) -> DateTime<Utc> {
        self.start_time
    }

    pub fn storage(&self) -> Storage<'_> {
        Storage::new(self.pool)
    }

    pub fn pool(&self) -> &PgPool {
        self.pool
    }

    pub fn routing(&self) -> crate::RoutingPolicy {
        self.routing
    }
}

#[derive(Clone)]
pub struct EventContext<'a> {
    ctx: &'a Context<'a>,
    delivery: &'a mage_amqp::lapin::message::Delivery,
    event: &'a mage_types::events::Event,
}

impl<'a> EventContext<'a> {
    pub fn new(
        ctx: &'a Context,
        delivery: &'a mage_amqp::lapin::message::Delivery,
        event: &'a mage_types::events::Event,
    ) -> Self {
        Self { ctx, delivery, event }
    }

    pub fn event(&self) -> &mage_types::events::Event {
        self.event
    }

    pub async fn ack(&self) -> ::mage_error::Result<()> {
        self.delivery
            .ack(mage_amqp::lapin::options::BasicAckOptions::default())
            .await?;
        Ok(())
    }

    pub async fn nack(&self) -> ::mage_error::Result<()> {
        self.delivery
            .nack(mage_amqp::lapin::options::BasicNackOptions {
                multiple: false,
                requeue: true,
            })
            .await?;
        Ok(())
    }

    pub async fn reject(&self) -> ::mage_error::Result<()> {
        self.delivery
            .reject(mage_amqp::lapin::options::BasicRejectOptions { requeue: false })
            .await?;
        Ok(())
    }

    pub async fn enqueue(
        &self,
        key: impl std::fmt::Display,
        body: impl Into<mage_types::events::Data>,
    ) -> ::mage_error::Result<()> {
        let data = body.into();
        let event = self
            .storage()
            .events()
            .create(
                data.actor_id(),
                data.chat_id(),
                data.message_id(),
                data.task_id(),
                mage_types::events::new(self.event.tenant_id, self.event.trace_id, key, data),
            )
            .await?;

        self.socket.produce().enqueue(event).await?;
        Ok(())
    }
}

impl<'a> std::ops::Deref for EventContext<'a> {
    type Target = Context<'a>;

    fn deref(&self) -> &Self::Target {
        self.ctx
    }
}
