use askama::Template;
use axum::http::{StatusCode, header};
use axum::response::{Html, IntoResponse, Response as HttpResponse};

use crate::state;

#[derive(Clone, Template, serde::Serialize, serde::Deserialize)]
#[template(path = "console/index.html")]
struct ConsoleTemplate {
    tenant_id: uuid::Uuid,
    high_water_cursor: Option<uuid::Uuid>,
    reducer_version: u32,
}

pub async fn page(session: state::http::HttpSession) -> mage_error::Result<HttpResponse> {
    let tenant_id = session.config().console.tenant_id.unwrap();
    let high_water_cursor = session
        .storage()
        .events()
        .get(mage_storage::events::query::new().tenant(tenant_id).limit(1))
        .await?
        .items
        .first()
        .map(|event| event.id);
    let template = ConsoleTemplate {
        tenant_id,
        high_water_cursor,
        reducer_version: 2,
    };

    let body = template.render().map_err(mage_error::http)?;

    Ok((StatusCode::OK, [(header::CACHE_CONTROL, "no-store")], Html(body)).into_response())
}
