#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    Json,
    SafeTensors,
    Text,
    #[default]
    #[serde(other)]
    Unknown,
}

impl Format {
    pub fn from_ext(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "json" => Self::Json,
            "safetensors" => Self::SafeTensors,
            "txt" => Self::Text,
            _ => Self::Unknown,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::SafeTensors => "safetensors",
            Self::Text => "txt",
            Self::Unknown => "??",
        }
    }
}

impl std::fmt::Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
