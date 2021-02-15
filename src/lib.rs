use crate::property::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::{Display, Error, Formatter};

#[cfg(test)]
#[macro_use(quickcheck)]
extern crate quickcheck_macros;

#[cfg(feature = "enable_args")]
pub mod args;
pub mod env;
pub mod environment;
pub mod property;
#[cfg(feature = "enable_toml")]
pub mod toml;

#[derive(Clone)]
pub enum Property {
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
}

#[derive(Debug, PartialEq, Eq)]
pub enum PropertyError {
    ParseFail(String),
    RecursiveParse(String),
}

impl Display for PropertyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match self {
            PropertyError::ParseFail(e) => write!(f, "{}", e),
            PropertyError::RecursiveParse(n) => write!(f, "Recursive parsing property {}.", &n),
        }
    }
}

pub trait PropertySource {
    fn name(&self) -> String;
    fn get_property(&self, name: &str) -> Option<Property>;
    fn contains_property(&self, name: &str) -> bool {
        self.get_property(name).is_some()
    }
    fn is_empty(&self) -> bool;
}

pub struct MapPropertySource {
    name: String,
    map: HashMap<String, Property>,
}

impl MapPropertySource {
    pub fn new(name: String, map: HashMap<String, Property>) -> Self {
        MapPropertySource { name, map }
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

pub trait Environment: Sized {
    fn contains(&self, name: &str) -> bool {
        self.require::<Property>(name).is_ok()
    }
    fn require<T: FromProperty>(&self, name: &str) -> Result<T, PropertyError>;
    fn get<T: FromProperty>(&self, name: &str) -> Option<T> {
        self.require(name).ok()
    }
    fn get_or<T: FromProperty>(&self, name: &str, default: T) -> T {
        self.get(name).unwrap_or(default)
    }
}
