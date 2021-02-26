//! Provide yaml [`PropertySource`].
use crate::source::*;
use crate::*;
use std::path::PathBuf;
use yaml_rust::ScanError;

/// [`PropertySource`] read properties from yaml file.
#[derive(Debug, Copy, Clone)]
pub struct Yaml;

struct YamlItem {
    name: String,
    path: PathBuf,
    value: yaml_rust::yaml::Yaml,
}
impl YamlItem {
    fn new(path: PathBuf) -> Result<Self, PropertyError> {
        Ok(YamlItem {
            name: path.display().to_string(),
            path: path.clone(),
            value: yaml_rust::YamlLoader::load_from_str(&std::fs::read_to_string(path)?)?
                .pop()
                .ok_or(PropertyError::ParseFail("Empty yaml".to_owned()))?,
        })
    }
}

impl FileToPropertySource for Yaml {
    fn to_property_source(
        &self,
        path: PathBuf,
    ) -> Result<Option<Box<(dyn PropertySource)>>, PropertyError> {
        Ok(Some(Box::new(YamlItem::new(path)?)))
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

    fn get_keys(&self, prefix: &str) -> Vec<String> {
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
    fn load(&self) -> Result<Option<Box<dyn PropertySource>>, PropertyError> {
        Yaml.to_property_source(self.path.clone())
    }
}

impl From<ScanError> for PropertyError {
    fn from(err: ScanError) -> Self {
        Self::ParseFail(err.to_string())
    }
}
