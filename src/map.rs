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
    /// Create empty [`MapPropertySource`].
    pub fn empty(name: &str) -> Self {
        Self::new(name.to_owned(), HashMap::new())
    }

    /// Create a new [`MapPropertySource`].
    pub fn new(name: String, map: HashMap<String, Property>) -> Self {
        MapPropertySource { name, map }
    }

    /// Add property to [`MapPropertySource`].
    pub fn insert<T: IntoProperty>(&mut self, name: String, value: T) {
        self.map.insert(name, value.into_property());
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
}
