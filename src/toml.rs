//! Provide toml [`PropertySource`].
use crate::*;
use ::toml::*;
use std::env::*;
use std::fs;
use std::path::PathBuf;

/// Support read toml file as [`PropertySource`].
pub struct Toml {
    dir: Option<String>,
    name: String,
}

struct TomlItem {
    name: String,
    value: Value,
}

impl PropertySource for TomlItem {
    fn name(&self) -> String {
        self.name.to_owned()
    }
    fn get_property(&self, name: &str) -> Option<Property> {
        let mut v = &self.value;
        for n in name.split(".") {
            match v {
                Value::Table(t) => v = t.get(n)?,
                _ => return None,
            }
        }
        match v {
            Value::String(vs) => Some(Property::Str(vs.to_owned())),
            Value::Integer(vs) => Some(Property::Int(*vs)),
            Value::Float(vs) => Some(Property::Float(*vs)),
            Value::Boolean(vs) => Some(Property::Bool(*vs)),
            // Not Support Date, Array
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

impl Toml {
    /// Create toml environment.
    pub fn new(dir: Option<String>, name: String) -> Self {
        Self { dir, name }
    }

    /// Build and load toml [`PropertySource`].
    pub fn build(&self) -> Vec<Option<Box<dyn PropertySource>>> {
        let filename = format!("{}.toml", self.name);

        let mut v = vec![];
        if let Some(dir) = &self.dir {
            // Only load from specified location
            v.push(Self::load(Some(PathBuf::from(dir)), &filename));
        } else {
            // Load from current location
            let current = current_dir().ok();
            // Load from HOME location
            let mut home = var("HOME").ok().map(PathBuf::from);
            if current == home {
                home = None;
            }
            v.push(Self::load(current, &filename));
            v.push(Self::load(home, &filename));
        }
        v
    }

    fn load(dir: Option<PathBuf>, file: &str) -> Option<Box<dyn PropertySource>> {
        let mut dir = dir?;
        dir.push(file);
        Some(Box::new(TomlItem {
            name: dir.display().to_string(),
            value: from_str(&fs::read_to_string(dir).ok()?).ok()?,
        }))
    }
}
