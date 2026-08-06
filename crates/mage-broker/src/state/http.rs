use std::sync::Arc;
use std::time::Instant;

use axum::extract::{FromRequestParts, Request, State};
use axum::http::request::Parts;
use axum::middleware::Next;
use axum::response::Response;

const REQUEST_ID_HEADER: &str = "X-Request-ID";

#[derive(Clone)]
pub struct HttpSession {
    request_id: uuid::Uuid,
    parent: Arc<super::Session>,
    span: tracing::Span,
}

impl HttpSession {
    pub fn parent(&self) -> &super::Session {
        &self.parent
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

    #[tracing::instrument(
        level = "info",
        name = "event.enqueue",
        parent = self.span(),
        skip_all,
        fields(
            event_key = %key,
            event_id = tracing::field::Empty,
            trace_id = %trace_id,
            tenant_id = %tenant_id,
            actor_id = tracing::field::Empty,
            chat_id = tracing::field::Empty,
            message_id = tracing::field::Empty,
            task_id = tracing::field::Empty,
        )
    )]
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
        let span = tracing::Span::current();

        span.record("event_id", tracing::field::display(event.id));
        span.record("actor_id", tracing::field::debug(actor_id));
        span.record("chat_id", tracing::field::debug(chat_id));
        span.record("message_id", tracing::field::debug(message_id));
        span.record("task_id", tracing::field::debug(task_id));

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

        if let Err(error) = self.amqp.produce().enqueue(event.clone()).await {
            tracing::error!(%error, "failed to publish event to RabbitMQ");
            return Err(error);
        }

        tracing::debug!("published event to RabbitMQ");
        Ok(event)
    }
}

impl<S> FromRequestParts<S> for HttpSession
where
    S: Send + Sync,
{
    type Rejection = mage_error::Error;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let ctx = parts
            .extensions
            .get::<HttpSession>()
            .cloned()
            .expect("HttpSession not found in request extensions");

        Ok(ctx)
    }
}

impl std::ops::Deref for HttpSession {
    type Target = super::Session;

    fn deref(&self) -> &Self::Target {
        self.parent()
    }
}

#[tracing::instrument(
    level = "info",
    name = "http.request",
    skip_all,
    fields(
        request_id = tracing::field::Empty,
        method = tracing::field::Empty,
        path = tracing::field::Empty,
        status = tracing::field::Empty,
        elapsed_ms = tracing::field::Empty,
    )
)]
pub async fn middleware(State(parent): State<Arc<super::Session>>, mut request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let headers = request.headers().clone();
    let request_id = headers
        .get(REQUEST_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse().ok())
        .unwrap_or_else(uuid::Uuid::now_v7);

    let span = tracing::Span::current();
    span.record("request_id", tracing::field::display(request_id));
    span.record("method", tracing::field::display(&method));
    span.record("path", tracing::field::display(&path));

    request.extensions_mut().insert(HttpSession {
        parent,
        request_id,
        span: span.clone(),
    });

    let started_at = Instant::now();
    tracing::debug!("request started");
    let response = next.run(request).await;
    let elapsed_ms = started_at.elapsed().as_millis() as u64;
    let status = response.status().as_u16();
    span.record("status", status);
    span.record("elapsed_ms", elapsed_ms);

    if status >= 500 {
        tracing::error!("request completed");
    } else {
        tracing::info!("request completed");
    }

    response
}
