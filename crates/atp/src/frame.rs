use serde_valid::Validate;

use crate::{connect, message, stream};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Frame<T = serde_json::Value> {
    Request(Request<T>),
    Response(Response<T>),
    Event(Event<T>),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Request<T = serde_json::Value> {
    pub id: uuid::Uuid,
    pub method: String,
    pub params: T,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum Response<T = serde_json::Value> {
    Ok { id: uuid::Uuid, result: T },
    Err { id: uuid::Uuid, error: Error },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Event<T = serde_json::Value> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<uuid::Uuid>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence: Option<u64>,

    #[serde(flatten)]
    pub body: T,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Error {
    pub code: i64,
    pub message: String,
}

impl Error {
    pub const PARSE: i64 = -32700;
    pub const INVALID_REQUEST: i64 = -32600;
    pub const METHOD_NOT_FOUND: i64 = -32601;
    pub const INVALID_PARAMS: i64 = -32602;
    pub const INTERNAL: i64 = -32603;

    pub fn parse(message: impl std::fmt::Display) -> Self {
        Self {
            code: Self::PARSE,
            message: message.to_string(),
        }
    }

    pub fn invalid_request(message: impl std::fmt::Display) -> Self {
        Self {
            code: Self::INVALID_REQUEST,
            message: message.to_string(),
        }
    }

    pub fn method_not_found(message: impl std::fmt::Display) -> Self {
        Self {
            code: Self::METHOD_NOT_FOUND,
            message: message.to_string(),
        }
    }

    pub fn invalid_params(message: impl std::fmt::Display) -> Self {
        Self {
            code: Self::INVALID_PARAMS,
            message: message.to_string(),
        }
    }

    pub fn internal(message: impl std::fmt::Display) -> Self {
        Self {
            code: Self::INTERNAL,
            message: message.to_string(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Validate)]
#[serde(tag = "type")]
pub enum AtpPacket {
    #[serde(rename = "ack")]
    Ack,

    #[serde(rename = "connect")]
    #[validate]
    Connect(connect::Connect),

    #[serde(rename = "message")]
    #[validate]
    Message(message::Message),

    #[serde(untagged)]
    #[validate]
    Stream(stream::StreamEvent),
}

impl AtpPacket {
    pub fn as_connect(&self) -> Option<&connect::Connect> {
        match self {
            Self::Connect(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_message(&self) -> Option<&message::Message> {
        match self {
            Self::Message(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_stream_event(&self) -> Option<&stream::StreamEvent> {
        match self {
            Self::Stream(v) => Some(v),
            _ => None,
        }
    }
}

impl From<connect::Connect> for AtpPacket {
    fn from(value: connect::Connect) -> Self {
        Self::Connect(value)
    }
}

impl From<message::Message> for AtpPacket {
    fn from(value: message::Message) -> Self {
        Self::Message(value)
    }
}

impl From<stream::StreamEvent> for AtpPacket {
    fn from(value: stream::StreamEvent) -> Self {
        Self::Stream(value)
    }
}
