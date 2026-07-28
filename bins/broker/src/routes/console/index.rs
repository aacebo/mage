use askama::Template;
use axum::http::{StatusCode, header};
use axum::response::{Html, IntoResponse, Response as HttpResponse};

use crate::RequestContext;

#[derive(Clone, Template, serde::Serialize, serde::Deserialize)]
#[template(path = "console/index.html")]
struct ConsoleTemplate {
    tenant_id: uuid::Uuid,
    high_water_cursor: Option<uuid::Uuid>,
    reducer_version: u32,
}

pub async fn page(ctx: RequestContext) -> error::Result<HttpResponse> {
    let tenant_id = ctx.console().tenant_id.unwrap();
    let high_water_cursor = ctx
        .storage()
        .events()
        .get(storage::events::query::new().tenant(tenant_id).limit(1))
        .await?
        .items
        .first()
        .map(|event| event.id);
    let template = ConsoleTemplate {
        tenant_id,
        high_water_cursor,
        reducer_version: 2,
    };

    let body = template.render().map_err(error::http)?;

    Ok((StatusCode::OK, [(header::CACHE_CONTROL, "no-store")], Html(body)).into_response())
}
