use toml::Value;

use crate::{
    source_raw::FileItem, Key, Property, PropertyError, PropertySource, Res, SubKey, SubKeys,
};

#[derive(Debug)]
pub(crate) struct Toml {
    item: FileItem,
    name: String,
    value: Value,
}

impl Toml {
    pub(crate) fn new(item: FileItem) -> Res<Self> {
        Ok(Toml {
            name: item.name(),
            value: toml::from_str(&item.load()?)?,
            item,
        })
    }
}

fn sub_value<'a>(toml: &'a Toml, key: &Key<'_>) -> Option<&'a Value> {
    let mut val = &toml.value;
    for n in key.iter() {
        match n {
            SubKey::S(n) => match val {
                Value::Table(t) => val = t.get(*n)?,
                _ => return None,
            },
            SubKey::I(n) => match val {
                Value::Array(vs) => val = vs.get(*n)?,
                _ => return None,
            },
        }
    }
    Some(val)
}

impl PropertySource for Toml {
    fn name(&self) -> &str {
        &self.name
    }

    fn get_property(&self, key: &Key<'_>) -> Option<Property<'_>> {
        match sub_value(self, key)? {
            Value::String(vs) => Some(Property::S(vs)),
            Value::Integer(vs) => Some(Property::I(*vs)),
            Value::Float(vs) => Some(Property::F(*vs)),
            Value::Boolean(vs) => Some(Property::B(*vs)),
            Value::Datetime(vs) => Some(Property::O(vs.to_string())),
            _ => None,
        }
    }

    fn get_sub_keys<'a>(&'a self, key: &Key<'_>, sub_keys: &mut SubKeys<'a>) {
        match sub_value(self, key) {
            Some(Value::Table(t)) => t.keys().for_each(|f| sub_keys.insert(f.as_str())),
            Some(Value::Array(vs)) => sub_keys.insert(vs.len()),
            _ => {}
        }
    }

    fn is_empty(&self) -> bool {
        match &self.value {
            Value::Table(t) => t.is_empty(),
            _ => false,
        }
    }

    fn reload_source(&self) -> Result<Option<Box<dyn PropertySource>>, PropertyError> {
        Ok(Some(Box::new(Toml::new(self.item.clone())?)))
    }
}

/// Inline toml file as [`PropertySource`].
#[cfg_attr(docsrs, doc(cfg(feature = "toml")))]
#[macro_export]
macro_rules! inline_toml {
    ($x:expr) => {
        $crate::Toml::new(format!("inline_toml:{}", $x), include_str!($x)).unwrap()
    };
}
