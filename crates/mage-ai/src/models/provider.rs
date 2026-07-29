use std::str::FromStr;

use mage_error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Provider {
    #[serde(rename = "openai")]
    OpenAI,
}

impl Provider {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::OpenAI => "openai",
        }
    }

    pub const fn base_url(self) -> &'static str {
        match self {
            Self::OpenAI => "https://api.openai.com/v1",
        }
    }

    pub const fn env(self) -> &'static str {
        match self {
            Self::OpenAI => "OPENAI_API_KEY",
        }
    }
}

impl FromStr for Provider {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        match value.to_lowercase().replace(['-', '_'], "").as_str() {
            "openai" => Ok(Self::OpenAI),
            _ => Err(mage_error::parse(format!("unknown provider: {value:?}"))),
        }
    }
}

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_providers() {
        assert_eq!("openai".parse::<Provider>().unwrap(), Provider::OpenAI);
        assert_eq!("OpenAI".parse::<Provider>().unwrap(), Provider::OpenAI);
    }

    /// A typo must not silently fall through to a default client.
    #[test]
    fn rejects_an_unknown_provider() {
        assert!("anthropic".parse::<Provider>().is_err());
    }

    /// A weight registry is not an inference host.
    #[test]
    fn rejects_a_weight_registry() {
        assert!("huggingface".parse::<Provider>().is_err());
    }

    /// The wire value must stay `"openai"` -- a derived `rename_all` would emit `"open_ai"`.
    #[test]
    fn serde_uses_the_provider_name() {
        assert_eq!(serde_json::to_string(&Provider::OpenAI).unwrap(), "\"openai\"");
        assert_eq!(serde_json::from_str::<Provider>("\"openai\"").unwrap(), Provider::OpenAI);
    }
}
