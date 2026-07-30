mod notification;
mod request;
mod response;

pub use notification::*;
pub use request::*;
pub use response::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum Frame<T = serde_json::Value> {
    Request(Request<T>),
    Response(Response<T>),
    Notification(Notification<T>),
}

impl<T> From<Request<T>> for Frame<T> {
    fn from(value: Request<T>) -> Self {
        Self::Request(value)
    }
}

impl<T> From<Response<T>> for Frame<T> {
    fn from(value: Response<T>) -> Self {
        Self::Response(value)
    }
}

impl<T> From<Notification<T>> for Frame<T> {
    fn from(value: Notification<T>) -> Self {
        Self::Notification(value)
    }
}
