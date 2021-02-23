//! Provide toml [`PropertySource`].
use crate::source::*;
use crate::*;
use ::toml::{from_str, Value};
use std::path::PathBuf;

/// [`PropertySource`] read properties from toml file.
#[derive(Debug, Copy, Clone)]
pub struct Toml;

struct TomlItem {
    name: String,
    value: Value,
}

impl FileToPropertySource for Toml {
    fn to_property_source(&self, path: PathBuf) -> Option<Box<(dyn PropertySource)>> {
        Some(Box::new(TomlItem {
            name: path.display().to_string(),
            value: from_str(&std::fs::read_to_string(path).ok()?).ok()?,
        }))
    }
    fn extention(&self) -> &'static str {
        "toml"
    }
}

impl PropertySource for TomlItem {
    fn name(&self) -> String {
        self.name.to_owned()
    }
    fn get_property(&self, name: &str) -> Option<Property> {
        let mut v = &self.value;
        lazy_static::lazy_static! {
            static ref P: &'static [char] = &['.', '[', ']'];
        }
        for n in name.split(&P[..]) {
            if n.is_empty() {
                continue;
            }
            match v {
                Value::Table(t) => v = t.get(n)?,
                Value::Array(vs) => v = vs.get(n.parse::<usize>().ok()?)?,
                _ => return None,
            }
        }
        match v {
            Value::String(vs) => Some(Property::Str(vs.to_owned())),
            Value::Integer(vs) => Some(Property::Int(*vs)),
            Value::Float(vs) => Some(Property::Float(*vs)),
            Value::Boolean(vs) => Some(Property::Bool(*vs)),
            Value::Datetime(vs) => Some(Property::Str(vs.to_string())),
            _ => None,
        }
    }
    fn is_empty(&self) -> bool {
        match &self.value {
            Value::Table(t) => t.is_empty(),
            _ => false,
        }
    }
}
