//! Provide system environment [`PropertySource`].
use crate::utils::SalakStringUtil;
use crate::*;
use std::collections::BTreeMap;

/// [`PropertySource`] read properties from system environment.
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
    #[test]
    fn check_test() -> Result<(), PropertyError> {
        let mut env: Box<dyn PropertySource> = Box::new(SysEnvPropertySource::new());
        for i in 0..100 {
            std::env::set_var("hello", format!("{}", i));
            if let Some(e) = env.load()? {
                env = e;
            }
            let p = String::from_property(env.get_property("hello").unwrap())?;
            assert_eq!(format!("{}", i), p);
        }
        Ok(())
    }
}
