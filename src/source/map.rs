//! Provide hashmap [`PropertySource`].
use crate::*;
use std::collections::BTreeMap;

/// A simple implementation of [`PropertySource`].
#[derive(Debug, Clone)]
pub struct MapPropertySource {
    name: String,
    map: BTreeMap<String, Property>,
}

impl MapPropertySource {
    /// Create empty [`MapPropertySource`].
    pub fn empty(name: &str) -> Self {
        Self::new(name, BTreeMap::new())
    }

    /// Create a new [`MapPropertySource`].
    pub fn new(name: &str, map: BTreeMap<String, Property>) -> Self {
        MapPropertySource {
            name: name.to_owned(),
            map,
        }
    }
}

impl PropertySource for MapPropertySource {
    fn name(&self) -> String {
        self.name.to_owned()
    }

    fn contains_property(&self, name: &str) -> bool {
        self.map.contains_key(name)
    }
    fn get_property(&self, name: &str) -> Option<Property> {
        self.map.get(name).cloned()
    }
    fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
    fn find_keys(&self, prefix: &str) -> Vec<String> {
        if prefix.is_empty() {
            return self.map.keys().map(|k| (&k[..]).to_first()).collect();
        }
        self.map
            .range(format!("{}.", prefix)..format!("{}/", prefix))
            .into_iter()
            .flat_map(|(k, _)| k.strip_prefix(&format!("{}.", prefix)))
            .map(|k| k.to_first())
            .collect()
    }
}
