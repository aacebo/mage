#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Artifact {
    pub name: String,
    pub value: ArtifactContent,
    pub vector: Option<Vec<f32>>,
}

impl std::fmt::Display for Artifact {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.name, self.value)
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ArtifactContent {
    Text { text: String },
    File { path: std::path::PathBuf },
}

impl ArtifactContent {
    pub fn text(value: impl Into<String>) -> Self {
        Self::Text { text: value.into() }
    }

    pub fn file(path: std::path::PathBuf) -> Self {
        Self::File { path }
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text { text } => Some(text),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Text { text } => Some(text),
            Self::File { path } => path.to_str(),
        }
    }

    pub fn as_file(&self) -> Option<&std::path::PathBuf> {
        match self {
            Self::File { path } => Some(path),
            _ => None,
        }
    }
}

impl std::fmt::Display for ArtifactContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text { text } => write!(f, "{text}"),
            Self::File { path } => write!(f, "{}", path.display()),
        }
    }
}
