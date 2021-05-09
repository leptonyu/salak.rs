//! Provide system environment [`PropertySource`].
use crate::utils::SalakStringUtil;
use crate::*;
use std::collections::BTreeMap;

/// [`PropertySource`] read properties from system environment.
///
/// [`SysEnvPropertySource`] will convert key from `SNAKE_UPPERCASE` to `dot.lowercase`.
///
/// * `NAME_URL` => `name.url`
/// * `NAME__URL` => `name_url`
/// * `DATABASE_USER__NAME` => `database.user_name`
/// * `__CFBundleIdentifier` will not convert.
#[derive(Debug, Clone)]
pub struct SysEnvPropertySource(MapPropertySource);

impl SysEnvPropertySource {
    pub(crate) fn new() -> Self {
        let mut map = BTreeMap::new();
        for (k, v) in std::env::vars() {
            let k: &str = &k;
            let k2 = k.to_key();
            if k2 != k {
                map.insert(k.to_owned(), Property::Str(v.clone()));
            }
            map.insert(k2, Property::Str(v));
        }
        Self(MapPropertySource::new("SystemEnvironment", map))
    }
}

impl PropertySource for SysEnvPropertySource {
    fn name(&self) -> String {
        self.0.name()
    }
    fn get_property(&self, name: &str) -> Option<Property> {
        self.0.get_property(name)
    }
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    fn get_keys(&self, prefix: &str) -> Vec<String> {
        self.0.get_keys(prefix)
    }

    fn load(&self) -> Result<Option<Box<dyn PropertySource>>, PropertyError> {
        Ok(Some(Box::new(SysEnvPropertySource::new())))
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    use std::convert::TryInto;
    #[test]
    fn check_test() -> Result<(), PropertyError> {
        let mut env: Box<dyn PropertySource> = Box::new(SysEnvPropertySource::new());
        for i in 0..100 {
            std::env::set_var("hello", format!("{}", i));
            if let Some(e) = env.load()? {
                env = e;
            }
            let p: String = env.get_property("hello").unwrap().try_into()?;
            assert_eq!(format!("{}", i), p);
        }
        Ok(())
    }
}
