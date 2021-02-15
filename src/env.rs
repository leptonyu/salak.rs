//! Provide system environment property source.
use crate::*;

/// A wrapper of `PropertySource` for getting properties from system environment.
pub struct SysEnv;

impl PropertySource for SysEnv {
    fn name(&self) -> String {
        "SystemEnvironment".to_owned()
    }
    fn get_property(&self, name: &str) -> Option<Property> {
        std::env::var(name).ok().map(|a| Property::Str(a))
    }
    fn is_empty(&self) -> bool {
        false
    }
}
