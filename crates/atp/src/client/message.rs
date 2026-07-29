use serde_valid::Validate;

pub fn new(chat_id: impl Into<uuid::Uuid>) -> Builder {
    Builder {
        _chat_id: chat_id.into(),
        ..Default::default()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Validate)]
pub struct Message {
    pub chat_id: uuid::Uuid,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<uuid::Uuid>,

    #[validate]
    pub content: mage_types::data::Contents,

    #[serde(default)]
    pub metadata: mage_types::data::Metadata,
}

impl Message {
    pub fn into_signal(self) -> super::Signal {
        self.into()
    }
}

#[doc(hidden)]
#[derive(Clone, Default)]
pub struct Builder {
    _chat_id: uuid::Uuid,
    _reply_to: Option<uuid::Uuid>,
    _content: mage_types::data::Contents,
    _metadata: mage_types::data::Metadata,
}

impl Builder {
    pub fn reply_to(mut self, value: impl Into<uuid::Uuid>) -> Self {
        self._reply_to = Some(value.into());
        self
    }

    pub fn meta(mut self, key: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        self._metadata.set(key.into(), value.into());
        self
    }

    pub fn push(mut self, value: impl Into<mage_types::data::Content>) -> Self {
        self._content.push(value.into());
        self
    }

    pub fn build(self) -> Message {
        Message {
            chat_id: self._chat_id,
            reply_to: self._reply_to,
            content: self._content,
            metadata: self._metadata,
        }
    }
}
