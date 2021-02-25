//! Provide yaml [`PropertySource`].
use crate::source::*;
use crate::*;
use std::path::PathBuf;

/// [`PropertySource`] read properties from yaml file.
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
lazy_static::lazy_static! {
    static ref P: &'static [char] = &['.', '[', ']'];
}

impl PropertySource for YamlItem {
    fn name(&self) -> String {
        self.name.to_owned()
    }
    fn get_property(&self, name: &str) -> Option<Property> {
        use yaml_rust::yaml::Yaml;
        let mut v = &self.value;
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

    fn find_keys(&self, prefix: &str) -> Vec<String> {
        use yaml_rust::yaml::Yaml;
        let mut v = &self.value;
        for n in prefix.split(&P[..]) {
            if n.is_empty() {
                continue;
            }
            match v {
                Yaml::Hash(t) => {
                    if let Some(x) = t.get(&Yaml::String(n.to_string())) {
                        v = x;
                    } else {
                        return vec![];
                    }
                }
                _ => return vec![],
            }
        }
        match v {
            Yaml::Hash(t) => t
                .keys()
                .map(|x| match x {
                    Yaml::String(v) => Some(v.to_string()),
                    _ => None,
                })
                .flatten()
                .collect(),
            _ => vec![],
        }
    }
}
