use axum::extract::{FromRequest, Request};

#[derive(Debug)]
pub struct Json<T>(pub T);

impl<T> Json<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> std::ops::Deref for Json<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> std::ops::DerefMut for Json<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T, S> FromRequest<S> for Json<T>
where
    T: serde::de::DeserializeOwned + serde_valid::Validate + Send,
    S: Send + Sync,
{
    type Rejection = error::Error;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        let axum::Json(body) = axum::Json::<T>::from_request(request, state)
            .await
            .map_err(error::bad_request)?;
        body.validate()?;
        Ok(Self(body))
    }
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode, header};
    use axum::routing::post;
    use serde_valid::Validate;
    use tower::ServiceExt;

    #[derive(serde::Deserialize, Validate)]
    struct Payload {
        #[validate(minimum = 1)]
        value: usize,
    }

    async fn handler(super::Json(payload): super::Json<Payload>) -> StatusCode {
        assert!(payload.value > 0);
        StatusCode::NO_CONTENT
    }

    async fn response(body: &'static str) -> axum::response::Response {
        Router::new()
            .route("/", post(handler))
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn accepts_valid_json() {
        assert_eq!(response(r#"{"value":1}"#).await.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn rejects_invalid_and_unvalidated_json_as_bad_requests() {
        for body in ["{", r#"{"value":0}"#] {
            let response = response(body).await;
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
            let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
            let error: error::Error = serde_json::from_slice(&body).unwrap();
            assert_eq!(error.name(), "bad_request");
        }
    }
}
