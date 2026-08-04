pub mod client;
pub mod error;
pub mod server;
pub mod types;
pub mod wire;

pub use error::Error;

pub trait Socket {
    type Error;

    fn read<T>(&mut self) -> std::pin::Pin<Box<impl Future<Output = Result<Output<T>, Self::Error>>>>
    where
        T: for<'a> serde::Deserialize<'a>;

    fn write<T>(&mut self, item: T) -> std::pin::Pin<Box<impl Future<Output = Result<(), Self::Error>>>>
    where
        T: serde::Serialize;

    fn close(
        &mut self,
        code: CloseCode,
        reason: Option<impl std::fmt::Display>,
    ) -> std::pin::Pin<Box<impl Future<Output = Result<(), Self::Error>>>>;
}

#[derive(Debug, Clone)]
pub enum Output<T = serde_json::Value> {
    Frame(T),
    Continue,
    Close { code: CloseCode, message: Option<String> },
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

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(u16)]
#[serde(rename_all = "snake_case")]
pub enum CloseCode {
    #[default]
    Normal = 1000,
    InvalidData = 1007,
    Policy = 1008,
    InternalError = 1011,
}

impl CloseCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::InvalidData => "invalid data",
            Self::Policy => "policy",
            Self::InternalError => "internal error",
        }
    }
}

impl TryFrom<u16> for CloseCode {
    type Error = Error;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            1000 => Ok(Self::Normal),
            1007 => Ok(Self::InvalidData),
            1008 => Ok(Self::Policy),
            1011 => Ok(Self::InternalError),
            v => Err(error::protocol(format!("invalid close code `{v}`"))),
        }
    }
}

impl std::fmt::Display for CloseCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::Output;
    use crate::CloseCode;

    #[test]
    fn close_preserves_transport_details() {
        let output = Output::<serde_json::Value>::Close {
            code: CloseCode::Policy,
            message: Some("policy violation".to_string()),
        };

        assert!(output.is_close());
        assert!(!output.is_frame());
        assert!(!output.is_continue());
        let Output::Close { code, message } = output else {
            unreachable!();
        };
        assert_eq!(Ok(code), CloseCode::try_from(1008));
        assert_eq!(message.as_deref(), Some("policy violation"));
    }
}
