use crate::*;

pub(crate) mod factory;
pub(crate) mod internal;
pub(crate) mod placeholder;
pub(crate) mod registry;
pub(crate) mod salak;

impl<P: PropertySource> Environment for P {
    fn require<T: FromEnvironment>(&self, name: &str) -> Result<T, PropertyError> {
        T::from_env(name, self.get_property(name), self)
    }
    fn resolve_placeholder(&self, _: String) -> Result<Option<Property>, PropertyError> {
        Err(PropertyError::parse_failed("Not implement"))
    }
    fn find_keys(&self, prefix: &str) -> Vec<String> {
        self.get_keys(prefix)
    }
    fn reload(&mut self) -> Result<(), PropertyError> {
        Err(PropertyError::ReloadFail("Reload not supported".to_owned()))
    }
}
