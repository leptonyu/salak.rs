//! Provide toml [`PropertySource`].
use crate::source::*;
use crate::*;
use ::toml::{from_str, Value};
use std::path::PathBuf;

/// [`PropertySource`] read properties from toml file.
#[cfg_attr(docsrs, doc(cfg(feature = "enable_toml")))]
#[derive(Debug, Copy, Clone)]
pub struct Toml;

struct TomlItem {
    name: String,
    path: PathBuf,
    value: Value,
}

impl TomlItem {
    fn new(path: PathBuf) -> Result<Self, PropertyError> {
        Ok(TomlItem {
            name: path.display().to_string(),
            path: path.clone(),
            value: from_str(&std::fs::read_to_string(path)?)?,
        })
    }
}

impl FileToPropertySource for Toml {
    fn to_property_source(
        &self,
        path: PathBuf,
    ) -> Result<Option<Box<(dyn PropertySource)>>, PropertyError> {
        Ok(Some(Box::new(TomlItem::new(path)?)))
    }
    fn extention(&self) -> &'static str {
        "toml"
    }
}

lazy_static::lazy_static! {
    static ref P: &'static [char] = &['.', '[', ']'];
}

impl PropertySource for TomlItem {
    fn name(&self) -> String {
        self.name.to_owned()
    }
    fn get_property(&self, name: &str) -> Option<Property> {
        let mut v = &self.value;
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
    fn get_keys(&self, prefix: &str) -> Vec<String> {
        let mut v = &self.value;
        for n in prefix.split(&P[..]) {
            if n.is_empty() {
                continue;
            }
            match v {
                Value::Table(t) => {
                    if let Some(x) = t.get(n) {
                        v = x;
                    } else {
                        return vec![];
                    }
                }
                _ => return vec![],
            }
        }
        match v {
            Value::Table(t) => t.keys().map(|x| x.to_string()).collect(),
            _ => vec![],
        }
    }
    fn load(&self) -> Result<Option<Box<dyn PropertySource>>, PropertyError> {
        Toml.to_property_source(self.path.clone())
    }
}

impl From<::toml::value::DatetimeParseError> for PropertyError {
    fn from(err: ::toml::value::DatetimeParseError) -> Self {
        Self::ParseFail(err.to_string())
    }
}

impl From<::toml::de::Error> for PropertyError {
    fn from(err: ::toml::de::Error) -> Self {
        Self::ParseFail(err.to_string())
    }
}