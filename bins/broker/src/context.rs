use std::future::{Ready, ready};
use std::sync::Arc;
use std::time::Instant;

use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready};
use actix_web::http::header::HeaderMap;
use actix_web::{Error as ActixError, FromRequest, HttpMessage, HttpRequest, web};
use chrono::{DateTime, Utc};
use futures_util::future::LocalBoxFuture;
use sqlx::PgPool;
use storage::Storage;
use tracing::Instrument;

const REQUEST_ID_HEADER: &str = "X-Request-ID";

#[derive(Clone)]
pub struct Context {
    pool: PgPool,
    socket: amqp::Socket,
    start_time: DateTime<Utc>,
    console: crate::ConsoleConfig,
}

impl Context {
    pub fn new(pool: PgPool, socket: amqp::Socket, console: crate::ConsoleConfig) -> Self {
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

    pub fn socket(&self) -> &amqp::Socket {
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
        body: impl Into<types::events::Data>,
    ) -> ::error::Result<types::events::Event> {
        self.enqueue_with_trace(tenant_id, self.request_id, key, body).await
    }

    pub async fn enqueue_with_trace(
        &self,
        tenant_id: uuid::Uuid,
        trace_id: uuid::Uuid,
        key: impl std::fmt::Display,
        body: impl Into<types::events::Data>,
    ) -> ::error::Result<types::events::Event> {
        let data = body.into();
        let actor_id = data.actor_id();
        let chat_id = data.chat_id();
        let message_id = data.message_id();
        let task_id = data.task_id();
        let event = types::events::new(tenant_id, trace_id, key, data);
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

impl FromRequest for RequestContext {
    type Error = error::Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut actix_web::dev::Payload) -> Self::Future {
        let ctx = req
            .extensions()
            .get::<RequestContext>()
            .cloned()
            .expect("RequestContext not found in request extensions");

        ready(Ok(ctx))
    }
}

impl std::ops::Deref for RequestContext {
    type Target = Context;

    fn deref(&self) -> &Self::Target {
        self.context()
    }
}

pub struct RequestMiddleware;

impl<S, B> Transform<S, ServiceRequest> for RequestMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = ActixError>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = ActixError;
    type Transform = RequestMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(Self::Transform { service }))
    }
}

pub struct RequestMiddlewareService<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for RequestMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = ActixError>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = ActixError;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let method = req.method().clone();
        let path = req.path().to_string();
        let ctx = req
            .app_data::<web::Data<Context>>()
            .expect("Context not found in app data")
            .clone()
            .into_inner();

        let headers = req.headers().clone();
        let request_id = headers
            .get(REQUEST_ID_HEADER)
            .and_then(|v| v.to_str().ok())
            .map(String::from)
            .unwrap_or_else(|| uuid::Uuid::now_v7().to_string())
            .parse()
            .unwrap_or_else(|_| uuid::Uuid::now_v7());

        let span = tracing::info_span!(
            "http.request",
            request_id = %request_id,
            method = %method,
            path = %path,
            status = tracing::field::Empty,
            elapsed_ms = tracing::field::Empty,
        );

        let ctx = RequestContext::new(ctx, headers, request_id, span.clone());
        req.extensions_mut().insert(ctx);
        let future = self.service.call(req);
        let completion_span = span.clone();

        Box::pin(
            async move {
                let started_at = Instant::now();
                tracing::debug!("request started");
                let result = future.await;
                let elapsed_ms = started_at.elapsed().as_millis() as u64;

                match &result {
                    Ok(response) => {
                        let status = response.status().as_u16();
                        completion_span.record("status", status);
                        completion_span.record("elapsed_ms", elapsed_ms);

                        if status >= 500 {
                            tracing::error!("request completed");
                        } else {
                            tracing::info!("request completed");
                        }
                    }
                    Err(error) => {
                        let status = error.as_response_error().status_code().as_u16();
                        completion_span.record("status", status);
                        completion_span.record("elapsed_ms", elapsed_ms);
                        tracing::error!(%error, "request failed");
                    }
                }

                result
            }
            .instrument(span),
        )
    }
}
