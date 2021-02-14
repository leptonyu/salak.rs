use crate::*;

pub struct SysEnv;

impl PropertySource for SysEnv {
    fn name(&self) -> &'static str {
        "SystemEnvironment"
    }
    fn get_property(&self, name: &str) -> Option<Property> {
        std::env::var(name).ok().map(|a| Property::Str(a))
    }
}
