//! Provide yaml [`PropertySource`].
use crate::file::FileToPropertySource;
use crate::*;
use std::path::PathBuf;

/// Support read yaml file as [`PropertySource`].
#[derive(Debug, Copy, Clone)]
pub struct Yaml;

struct YamlItem {
    name: String,
    value: yaml_rust::yaml::Yaml,
}

impl FileToPropertySource for Yaml {
    fn to_property_source(&self, path: PathBuf) -> Option<Box<(dyn PropertySource)>> {
        Some(Box::new(YamlItem {
            name: path.display().to_string(),
            value: yaml_rust::YamlLoader::load_from_str(&std::fs::read_to_string(path).ok()?)
                .ok()?
                .pop()?,
        }))
    }
    fn extention(&self) -> &'static str {
        "yaml"
    }
}

impl PropertySource for YamlItem {
    fn name(&self) -> String {
        self.name.to_owned()
    }
    fn get_property(&self, name: &str) -> Option<Property> {
        use yaml_rust::yaml::Yaml;
        let mut v = &self.value;
        lazy_static::lazy_static! {
            static ref P: &'static [char] = &['.', '[', ']'];
        }
        for n in name.split(&P[..]) {
            if n.is_empty() {
                continue;
            }
            match v {
                Yaml::Hash(t) => v = t.get(&Yaml::String(n.to_owned()))?,
                Yaml::Array(vs) => v = vs.get(n.parse::<usize>().ok()?)?,
                _ => return None,
            }
        }
        match v {
            Yaml::String(vs) => Some(Property::Str(vs.to_owned())),
            Yaml::Integer(vs) => Some(Property::Int(*vs)),
            Yaml::Real(vs) => Some(Property::Str(vs.to_owned())),
            Yaml::Boolean(vs) => Some(Property::Bool(*vs)),
            _ => None,
        }
    }
    fn is_empty(&self) -> bool {
        use yaml_rust::yaml::Yaml;
        match &self.value {
            Yaml::Hash(t) => t.is_empty(),
            _ => false,
        }
    }
}
