mod notification;
mod request;
mod response;

pub use notification::*;
pub use request::*;
pub use response::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum Frame<T = serde_json::Value> {
    Response(Response<T>),
    Request(Request<T>),
    Notification(Notification<T>),
}

impl<T> Frame<T> {
    pub fn is_request(&self) -> bool {
        matches!(self, Self::Request(_))
    }

    pub fn is_response(&self) -> bool {
        matches!(self, Self::Response(_))
    }

    pub fn is_notification(&self) -> bool {
        matches!(self, Self::Notification(_))
    }

    pub fn payload(&self) -> Result<&T, &Error> {
        match self {
            Self::Request(v) => Ok(&v.params),
            Self::Response(v) => v.payload(),
            Self::Notification(v) => Ok(&v.body),
        }
    }
}

impl Frame {
    pub fn try_cast_into<T>(self) -> Result<Frame<T>, crate::Error>
    where
        T: for<'a> serde::Deserialize<'a>,
    {
        match self {
            Self::Request(v) => Ok(v.try_cast_into()?.into()),
            Self::Response(v) => Ok(v.try_cast_into()?.into()),
            Self::Notification(v) => Ok(v.try_cast_into()?.into()),
        }
    }
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
