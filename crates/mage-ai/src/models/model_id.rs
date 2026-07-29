use std::str::FromStr;

use mage_error::{Error, Result};

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ModelId {
    group: Option<Box<str>>,
    name: Box<str>,
}

impl ModelId {
    pub fn group(&self) -> Option<&str> {
        self.group.as_deref()
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl FromStr for ModelId {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        let Some((group, name)) = value.split_once('/') else {
            if value.is_empty() {
                return Err(mage_error::parse(format!("invalid model id: {value:?}")));
            }

            return Ok(Self {
                group: None,
                name: value.into(),
            });
        };

        if group.is_empty() || name.is_empty() {
            return Err(mage_error::parse(format!("invalid model id: {value:?}")));
        }

        Ok(Self {
            group: Some(group.into()),
            name: name.into(),
        })
    }
}

impl std::fmt::Display for ModelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.group {
            Some(group) => write!(f, "{group}/{}", self.name),
            None => write!(f, "{}", self.name),
        }
    }
}

impl std::fmt::Debug for ModelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

impl serde::Serialize for ModelId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        self.to_string().serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for ModelId {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        Self::from_str(&value).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_grouped() {
        let id: ModelId = "facebook/bart-large-cnn".parse().unwrap();
        assert_eq!(id.group(), Some("facebook"));
        assert_eq!(id.name(), "bart-large-cnn");
        assert_eq!(id.to_string(), "facebook/bart-large-cnn");
    }

    #[test]
    fn parses_ungrouped() {
        let id: ModelId = "gpt-5".parse().unwrap();
        assert_eq!(id.group(), None);
        assert_eq!(id.name(), "gpt-5");
        assert_eq!(id.to_string(), "gpt-5");
    }

    #[test]
    fn splits_on_the_first_slash_only() {
        let id: ModelId = "facebook/bart/large".parse().unwrap();
        assert_eq!(id.group(), Some("facebook"));
        assert_eq!(id.name(), "bart/large");
    }

    #[test]
    fn rejects_empty_segments() {
        for value in ["", "/name", "group/"] {
            assert!(value.parse::<ModelId>().is_err(), "{value:?} should not parse");
        }
    }

    #[test]
    fn serde_round_trips() {
        let id: ModelId = "facebook/bart-large-cnn".parse().unwrap();
        let json = serde_json::to_string(&id).unwrap();

        assert_eq!(json, "\"facebook/bart-large-cnn\"");
        assert_eq!(serde_json::from_str::<ModelId>(&json).unwrap(), id);
    }
}
