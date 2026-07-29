use std::sync::Arc;
use std::time::Instant;

use axum::extract::{FromRequestParts, Request, State};
use axum::http::HeaderMap;
use axum::http::request::Parts;
use axum::middleware::Next;
use axum::response::Response;
use chrono::{DateTime, Utc};
use mage_storage::Storage;
use sqlx::PgPool;
use tracing::Instrument;

const REQUEST_ID_HEADER: &str = "X-Request-ID";

#[derive(Clone)]
pub struct Context {
    pool: PgPool,
    socket: mage_amqp::Socket,
    start_time: DateTime<Utc>,
    console: crate::ConsoleConfig,
}

impl Context {
    pub fn new(pool: PgPool, socket: mage_amqp::Socket, console: crate::ConsoleConfig) -> Self {
        Self {
            pool,
            socket,
            start_time: Utc::now(),
            console,
        }
    }

    pub fn start_time(&self) -> DateTime<Utc> {
        self.start_time
    }

    pub fn storage(&self) -> Storage<'_> {
        Storage::new(&self.pool)
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub fn socket(&self) -> &mage_amqp::Socket {
        &self.socket
    }

    pub fn console(&self) -> &crate::ConsoleConfig {
        &self.console
    }
}

#[derive(Clone)]
pub struct RequestContext {
    ctx: Arc<Context>,
    headers: HeaderMap,
    request_id: uuid::Uuid,
    span: tracing::Span,
}

impl RequestContext {
    pub fn new(ctx: Arc<Context>, headers: HeaderMap, request_id: uuid::Uuid, span: tracing::Span) -> Self {
        Self {
            ctx,
            headers,
            request_id,
            span,
        }
    }

    pub fn context(&self) -> &Context {
        &self.ctx
    }

    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    pub fn request_id(&self) -> &uuid::Uuid {
        &self.request_id
    }

    pub fn span(&self) -> &tracing::Span {
        &self.span
    }

    pub async fn enqueue(
        &self,
        tenant_id: uuid::Uuid,
        key: impl std::fmt::Display,
        body: impl Into<mage_types::events::Data>,
    ) -> ::mage_error::Result<mage_types::events::Event> {
        self.enqueue_with_trace(tenant_id, self.request_id, key, body).await
    }

    pub async fn enqueue_with_trace(
        &self,
        tenant_id: uuid::Uuid,
        trace_id: uuid::Uuid,
        key: impl std::fmt::Display,
        body: impl Into<mage_types::events::Data>,
    ) -> ::mage_error::Result<mage_types::events::Event> {
        let data = body.into();
        let actor_id = data.actor_id();
        let chat_id = data.chat_id();
        let message_id = data.message_id();
        let task_id = data.task_id();
        let event = mage_types::events::new(tenant_id, trace_id, key, data);
        let span = tracing::info_span!(
            parent: self.span(),
            "event.enqueue",
            event_key = %event.key,
            event_id = %event.id,
            trace_id = %event.trace_id,
            tenant_id = %event.tenant_id,
            actor_id = ?actor_id,
            chat_id = ?chat_id,
            message_id = ?message_id,
            task_id = ?task_id,
        );

        async {
            let event = match self
                .storage()
                .events()
                .create(actor_id, chat_id, message_id, task_id, event)
                .await
            {
                Ok(event) => event,
                Err(error) => {
                    tracing::error!(%error, "failed to persist event");
                    return Err(error);
                }
            };

            tracing::debug!("persisted event");

            if let Err(error) = self.socket.produce().enqueue(event.clone()).await {
                tracing::error!(%error, "failed to publish event to RabbitMQ");
                return Err(error);
            }

            tracing::debug!("published event to RabbitMQ");
            Ok(event)
        }
        .instrument(span)
        .await
    }
}

impl<S> FromRequestParts<S> for RequestContext
where
    S: Send + Sync,
{
    type Rejection = mage_error::Error;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let ctx = parts
            .extensions
            .get::<RequestContext>()
            .cloned()
            .expect("RequestContext not found in request extensions");

        Ok(ctx)
    }
}

impl std::ops::Deref for RequestContext {
    type Target = Context;

    fn deref(&self) -> &Self::Target {
        self.context()
    }
}

pub async fn request_middleware(State(ctx): State<Arc<Context>>, mut request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let headers = request.headers().clone();
    let request_id = headers
        .get(REQUEST_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse().ok())
        .unwrap_or_else(uuid::Uuid::now_v7);
    let span = tracing::info_span!(
        "http.request",
        request_id = %request_id,
        method = %method,
        path = %path,
        status = tracing::field::Empty,
        elapsed_ms = tracing::field::Empty,
    );

    request
        .extensions_mut()
        .insert(RequestContext::new(ctx, headers, request_id, span.clone()));
    let completion_span = span.clone();

    async move {
        let started_at = Instant::now();
        tracing::debug!("request started");
        let response = next.run(request).await;
        let elapsed_ms = started_at.elapsed().as_millis() as u64;
        let status = response.status().as_u16();
        completion_span.record("status", status);
        completion_span.record("elapsed_ms", elapsed_ms);

        if status >= 500 {
            tracing::error!("request completed");
        } else {
            tracing::info!("request completed");
        }

        response
    }
    .instrument(span)
    .await
}
