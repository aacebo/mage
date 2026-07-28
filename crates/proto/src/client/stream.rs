use serde_valid::Validate;

pub fn error(code: impl std::fmt::Display, message: impl std::fmt::Display) -> Builder {
    Builder {
        _stream_id: uuid::Uuid::now_v7().to_string(),
        _index: 0,
        _is_final: false,
        _item: None,
        _event: StreamEvent::error(code, message),
        _status: None,
    }
}

pub fn text(text: impl std::fmt::Display) -> Builder {
    Builder {
        _stream_id: uuid::Uuid::now_v7().to_string(),
        _index: 0,
        _is_final: false,
        _item: None,
        _event: StreamEvent::text(text),
        _status: None,
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Validate)]
pub struct StreamResponse {
    pub stream_id: String,
    pub index: usize,

    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_final: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub item: Option<usize>,

    #[serde(flatten)]
    #[validate]
    pub event: StreamEvent,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

impl StreamResponse {
    pub fn into_signal(self) -> super::Signal {
        self.into()
    }
}

impl std::ops::Deref for StreamResponse {
    type Target = StreamEvent;

    fn deref(&self) -> &Self::Target {
        &self.event
    }
}

impl std::ops::DerefMut for StreamResponse {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.event
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Validate)]
#[serde(tag = "type")]
pub enum StreamEvent {
    #[serde(rename = "response.stream.error")]
    Error { code: String, message: String },

    #[serde(rename = "response.stream.text")]
    Text { text: String },
}

impl StreamEvent {
    pub fn error(code: impl std::fmt::Display, message: impl std::fmt::Display) -> Self {
        Self::Error {
            code: code.to_string(),
            message: message.to_string(),
        }
    }

    pub fn text(text: impl std::fmt::Display) -> Self {
        Self::Text { text: text.to_string() }
    }
}

#[doc(hidden)]
#[derive(Clone)]
pub struct Builder {
    _stream_id: String,
    _index: usize,
    _is_final: bool,
    _item: Option<usize>,
    _event: StreamEvent,
    _status: Option<String>,
}

impl Builder {
    pub fn stream_id(mut self, value: impl std::fmt::Display) -> Self {
        self._stream_id = value.to_string();
        self
    }

    pub fn index(mut self, value: usize) -> Self {
        self._index = value;
        self
    }

    pub fn is_final(mut self) -> Self {
        self._is_final = true;
        self
    }

    pub fn item(mut self, value: usize) -> Self {
        self._item = Some(value);
        self
    }

    pub fn status(mut self, value: impl std::fmt::Display) -> Self {
        self._status = Some(value.to_string());
        self
    }

    pub fn build(self) -> StreamResponse {
        StreamResponse {
            stream_id: self._stream_id,
            index: self._index,
            is_final: self._is_final,
            item: self._item,
            event: self._event,
            status: self._status,
        }
    }
}
