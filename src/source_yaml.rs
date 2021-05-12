use yaml_rust::Yaml;

use crate::{
    source::{FileConfig, PropertyRegistry},
    Property, PropertyError, PropertySource,
};

struct YamlValue {
    name: String,
    value: Vec<Yaml>,
}

impl YamlValue {
    fn new(name: String, content: &str) -> Result<Self, PropertyError> {
        Ok(Self {
            name,
            value: yaml_rust::YamlLoader::load_from_str(content)?,
        })
    }
}
lazy_static::lazy_static! {
    static ref P: &'static [char] = &['.', '[', ']'];
}

impl PropertySource for YamlValue {
    fn name(&self) -> &str {
        &self.name
    }

    fn get_property(&self, key: &str) -> Option<Property<'_>> {
        for mut v in &self.value {
            for n in key.split(&P[..]) {
                if n.is_empty() {
                    continue;
                }
                match v {
                    Yaml::Hash(t) => v = t.get(&Yaml::String(n.to_owned()))?,
                    Yaml::Array(vs) => v = vs.get(n.parse::<usize>().ok()?)?,
                    _ => break,
                }
            }
            return match v {
                Yaml::String(vs) => Some(Property::S(vs)),
                Yaml::Integer(vs) => Some(Property::I(*vs)),
                Yaml::Real(vs) => Some(Property::S(vs)),
                Yaml::Boolean(vs) => Some(Property::B(*vs)),
                _ => continue,
            };
        }
        None
    }

    fn is_empty(&self) -> bool {
        for v in &self.value {
            return match v {
                Yaml::Hash(t) => t.is_empty(),
                _ => continue,
            };
        }
        false
    }
}

pub(crate) fn init_yaml(env: &mut PropertyRegistry, fc: &FileConfig) -> Result<(), PropertyError> {
    for p in fc.build("yaml", YamlValue::new)? {
        env.register_by_ref(p);
    }
    Ok(())
}
