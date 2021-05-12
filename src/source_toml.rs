use toml::Value;

use crate::{
    source::{FileConfig, PropertyRegistry},
    Property, PropertyError, PropertySource,
};

#[doc(hidden)]
pub struct Toml {
    name: String,
    value: Value,
}

impl Toml {
    #[doc(hidden)]
    pub fn new(name: String, content: &str) -> Result<Self, PropertyError> {
        Ok(Toml {
            name,
            value: toml::from_str(content)?,
        })
    }
}

lazy_static::lazy_static! {
    static ref P: &'static [char] = &['.', '[', ']'];
}

impl PropertySource for Toml {
    fn name(&self) -> &str {
        &self.name
    }

    fn get_property(&self, key: &str) -> Option<Property<'_>> {
        let mut v = &self.value;
        for n in key.split(&P[..]) {
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
            Value::String(vs) => Some(Property::S(vs)),
            Value::Integer(vs) => Some(Property::I(*vs)),
            Value::Float(vs) => Some(Property::F(*vs)),
            Value::Boolean(vs) => Some(Property::B(*vs)),
            Value::Datetime(vs) => Some(Property::O(vs.to_string())),
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

pub(crate) fn init_toml(env: &mut PropertyRegistry, fc: &FileConfig) -> Result<(), PropertyError> {
    for p in fc.build("toml", Toml::new)? {
        env.register_by_ref(p);
    }
    Ok(())
}

/// Inline toml file as [`PropertySource`].
#[cfg_attr(docsrs, doc(cfg(feature = "toml")))]
#[macro_export]
macro_rules! inline_toml {
    ($x:expr) => {
        Toml::new(format!("inline_toml:{}", $x), include_str!($x)).unwrap()
    };
}
