use serde_valid::Validate;

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize, Validate)]
#[serde(transparent)]
pub struct Contents(#[validate(min_items = 1)] Vec<Content>);

impl std::ops::Deref for Contents {
    type Target = Vec<Content>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Contents {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl std::fmt::Display for Contents {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for content in &self.0 {
            writeln!(f, "{content}")?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Content {
    Text {
        text: String,
    },
    File {
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,

        #[serde(flatten)]
        file: FileContent,
    },
    Json {
        json: serde_json::Value,
    },
}

impl Content {
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text { text } => Some(text),
            _ => None,
        }
    }

    pub fn as_file(&self) -> Option<(Option<&str>, &FileContent)> {
        match self {
            Self::File { name, file } => Some((name.as_deref(), file)),
            _ => None,
        }
    }

    pub fn as_json(&self) -> Option<&serde_json::Value> {
        match self {
            Self::Json { json } => Some(json),
            _ => None,
        }
    }
}

impl std::fmt::Display for Content {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text { text } => write!(f, "{text}"),
            Self::Json { json } => write!(f, "{json}"),
            Self::File { name, file } => match name {
                None => write!(f, "{file}"),
                Some(name) => write!(f, "{name}: {file}"),
            },
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileContent {
    Uri { uri: url::Url },
    Base64 { base64: String },
}

impl std::fmt::Display for FileContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Uri { uri } => write!(f, "{uri}"),
            Self::Base64 { base64 } => write!(f, "{base64}"),
        }
    }
}
