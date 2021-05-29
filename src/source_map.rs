use std::collections::HashMap;

use crate::{Key, Property, PropertySource, SubKeys};

/// An in-memory source, which is a string to string hashmap.
#[derive(Debug)]
pub struct HashMapSource {
    name: String,
    map: HashMap<String, String>,
}

impl HashMapSource {
    /// Create an in-memory source with a name.
    pub fn new(name: &'static str) -> Self {
        Self {
            name: name.to_owned(),
            map: HashMap::new(),
        }
    }

    /// Set property to the source.
    pub fn set<K: Into<String>, V: Into<String>>(mut self, key: K, val: V) -> Self {
        self.map.insert(key.into(), val.into());
        self
    }

    /// Set a batch of properties to the source.
    pub fn set_all(mut self, map: HashMap<String, String>) -> Self {
        self.map.extend(map);
        self
    }
}

impl PropertySource for HashMapSource {
    fn name(&self) -> &str {
        &self.name
    }

    fn get_property(&self, key: &Key<'_>) -> Option<Property<'_>> {
        self.map.get(key.as_str()).map(|s| Property::S(s))
    }

    fn get_sub_keys<'a>(&'a self, prefix: &Key<'_>, sub_keys: &mut SubKeys<'a>) {
        for key in self.map.keys() {
            if let Some(k) = key.strip_prefix(prefix.as_str()) {
                let pos = k.find('.').unwrap_or_else(|| k.len());
                sub_keys.insert(&k[0..pos]);
            }
        }
    }

    fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

/// Create source from system environment.
pub fn system_environment() -> HashMapSource {
    HashMapSource {
        name: "SystemEnvironment".to_owned(),
        map: std::env::vars().collect(),
    }
}
