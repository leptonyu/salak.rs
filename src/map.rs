//! Provide hashmap [`PropertySource`].
use crate::*;
use std::collections::HashMap;

/// A simple implementation of [`PropertySource`].
#[derive(Debug)]
pub struct MapPropertySource {
    name: String,
    map: HashMap<String, Property>,
}

impl MapPropertySource {
    /// Create a new [`MapPropertySource`].
    pub fn new(name: String, map: HashMap<String, Property>) -> Self {
        MapPropertySource { name, map }
    }

    pub(crate) fn insert(&mut self, map: HashMap<String, Property>) {
        for (k, v) in map.into_iter() {
            self.map.insert(k, v);
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
        self.map.get(name).map(|p| p.clone())
    }
    fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}
