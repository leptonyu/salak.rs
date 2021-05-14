use yaml_rust::Yaml;

use crate::{
    source::{FileConfig, PropertyRegistry},
    Key, Property, PropertyError, PropertySource, SubKey, SubKeys,
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

fn sub_value<'a>(mut val: &'a Yaml, key: &Key<'_>) -> Option<&'a Yaml> {
    for n in key.iter() {
        match n {
            SubKey::S(n) => match val {
                Yaml::Hash(t) => val = t.get(&Yaml::String(n.to_string()))?,
                _ => return None,
            },
            SubKey::I(n) => match val {
                Yaml::Array(vs) => val = vs.get(*n)?,
                _ => return None,
            },
        }
    }
    Some(val)
}

impl PropertySource for YamlValue {
    fn name(&self) -> &str {
        &self.name
    }

    fn get_property(&self, key: &Key<'_>) -> Option<Property<'_>> {
        for v in &self.value {
            if let Some(v) = sub_value(v, key) {
                return match v {
                    Yaml::String(vs) => Some(Property::S(vs)),
                    Yaml::Integer(vs) => Some(Property::I(*vs)),
                    Yaml::Real(vs) => Some(Property::S(vs)),
                    Yaml::Boolean(vs) => Some(Property::B(*vs)),
                    _ => continue,
                };
            }
        }
        None
    }

    fn sub_keys<'a>(&'a self, key: &Key<'_>, sub_keys: &mut SubKeys<'a>) {
        for v in &self.value {
            if let Some(v) = sub_value(v, key) {
                match v {
                    Yaml::Hash(t) => t.keys().for_each(|f| {
                        if let Some(v) = f.as_str() {
                            sub_keys.insert(v);
                        }
                    }),
                    Yaml::Array(vs) => sub_keys.insert(vs.len()),
                    _ => continue,
                }
            }
        }
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
