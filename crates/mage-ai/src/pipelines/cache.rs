use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use mage_error::Result;

use crate::models::ModelRef;

/// The api key is fingerprinted rather than stored, so credentials never sit in a long-lived map
/// yet two callers with different keys still get separate clients.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Key {
    model: ModelRef,
    credential: u64,
}

impl Key {
    pub fn new(model: &ModelRef, api_key: &Option<String>) -> Self {
        use std::hash::{Hash, Hasher};

        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        api_key.hash(&mut hasher);

        Self {
            model: model.clone(),
            credential: hasher.finish(),
        }
    }
}

pub struct Cache<T: ?Sized> {
    entries: RwLock<HashMap<Key, Arc<T>>>,
}

impl<T: ?Sized> Cache<T> {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
        }
    }

    pub fn len(&self) -> usize {
        self.entries.read().map(|entries| entries.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get_or_build(&self, key: Key, build: impl FnOnce() -> Result<Arc<T>>) -> Result<Arc<T>> {
        if let Ok(entries) = self.entries.read()
            && let Some(entry) = entries.get(&key)
        {
            return Ok(entry.clone());
        }

        // Built outside the write lock: loading a model is slow, and a duplicate build on a race
        // is cheaper than holding the lock across it.
        let entry = build()?;

        match self.entries.write() {
            Ok(mut entries) => Ok(entries.entry(key).or_insert(entry).clone()),
            Err(_) => Ok(entry),
        }
    }
}

impl<T: ?Sized> Default for Cache<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use crate::models::ModelId;

    fn model(repo: &str) -> ModelRef {
        ModelRef::hub(repo.parse::<ModelId>().unwrap())
    }

    /// One cache keyed by model, not one per capability: a model asked for twice loads its weights
    /// once and both callers get the same copy. Under a per-capability cache the build ran twice.
    #[test]
    fn one_key_builds_once() {
        let cache: Cache<u32> = Cache::new();
        let builds = AtomicUsize::new(0);
        let key = Key::new(&model("sentence-transformers/all-MiniLM-L12-v2"), &None);

        let build = || {
            builds.fetch_add(1, Ordering::SeqCst);
            Ok(Arc::new(7))
        };

        let first = cache.get_or_build(key.clone(), build).unwrap();
        let second = cache.get_or_build(key, build).unwrap();

        assert_eq!(builds.load(Ordering::SeqCst), 1);
        assert!(Arc::ptr_eq(&first, &second), "both callers must hold one copy");
    }

    #[test]
    fn a_different_model_is_a_different_entry() {
        let cache: Cache<u32> = Cache::new();
        let builds = AtomicUsize::new(0);

        let build = || {
            builds.fetch_add(1, Ordering::SeqCst);
            Ok(Arc::new(7))
        };

        cache
            .get_or_build(Key::new(&model("sentence-transformers/all-MiniLM-L12-v2"), &None), build)
            .unwrap();
        cache
            .get_or_build(Key::new(&model("sentence-transformers/all-MiniLM-L6-v2"), &None), build)
            .unwrap();

        assert_eq!(builds.load(Ordering::SeqCst), 2);
    }

    /// Two callers with different credentials get separate clients, even on the same model.
    #[test]
    fn a_different_api_key_is_a_different_entry() {
        let cache: Cache<u32> = Cache::new();
        let builds = AtomicUsize::new(0);
        let model = model("sentence-transformers/all-MiniLM-L12-v2");

        let build = || {
            builds.fetch_add(1, Ordering::SeqCst);
            Ok(Arc::new(7))
        };

        cache.get_or_build(Key::new(&model, &Some("one".into())), build).unwrap();
        cache.get_or_build(Key::new(&model, &Some("two".into())), build).unwrap();

        assert_eq!(builds.load(Ordering::SeqCst), 2);
    }
}
