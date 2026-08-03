pub mod client;
pub mod error;
pub mod server;
pub mod types;
pub mod wire;

pub use error::Error;

pub trait Socket {
    type Error;
    type In: for<'a> serde::Deserialize<'a>;
    type Out: serde::Serialize;

    fn read(&mut self) -> std::pin::Pin<Box<impl Future<Output = Result<Output<Self::In>, Self::Error>>>>;
    fn write(
        &mut self,
        item: impl Into<wire::Frame<Self::Out>>,
    ) -> std::pin::Pin<Box<impl Future<Output = Result<(), Self::Error>>>>;
    fn flush(&mut self) -> std::pin::Pin<Box<impl Future<Output = Result<usize, Self::Error>>>>;
    fn close(&mut self) -> std::pin::Pin<Box<impl Future<Output = Result<(), Self::Error>>>>;
}

#[derive(Debug, Clone)]
pub enum Output<T = serde_json::Value> {
    Frame(wire::Frame<T>),
    Continue,
    Close { code: u16, message: Option<String> },
}

impl<T> Output<T> {
    pub fn is_frame(&self) -> bool {
        matches!(self, Self::Frame(_))
    }

    pub fn is_continue(&self) -> bool {
        matches!(self, Self::Continue)
    }

    pub fn is_close(&self) -> bool {
        matches!(self, Self::Close { .. })
    }
}
