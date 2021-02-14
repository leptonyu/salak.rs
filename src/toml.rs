use crate::*;
use std::collections::HashMap;

pub struct Toml {
    name: String,
    map: HashMap<String, Property>,
}

impl PropertySource for Toml {
    fn name(&self) -> String {
        self.name.to_owned()
    }

    fn contains_property(&self, name: &str) -> bool {
        self.map.contains_key(name)
    }
    fn get_property(&self, name: &str) -> Option<Property> {
        self.map.get(name).map(|p| p.clone())
    }
}
