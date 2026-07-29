use std::collections::BTreeMap;

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct Metadata(BTreeMap<String, serde_json::Value>);

impl Metadata {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn exists(&self, key: &str) -> bool {
        self.0.contains_key(key)
    }

    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.0.get(key)
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut serde_json::Value> {
        self.0.get_mut(key)
    }

    pub fn set(&mut self, key: impl std::fmt::Display, value: impl Into<serde_json::Value>) -> &mut Self {
        self.0.insert(key.to_string(), value.into());
        self
    }
}
