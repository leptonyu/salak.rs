use std::collections::HashMap;

use crate::{
    normalize_key,
    source::{system_environment, FileConfig, MapProvider, PropertyRegistry},
    Environment, FromEnvironment, IsProperty, Property, PropertyError, PropertySource,
};
pub struct SalakBuilder {
    args: HashMap<String, String>,
}

impl SalakBuilder {
    pub fn set_args(mut self, args: HashMap<String, String>) -> Self {
        self.args.extend(args);
        self
    }

    pub fn set<K: Into<String>, V: Into<String>>(mut self, k: K, v: V) -> Self {
        self.args.insert(k.into(), v.into());
        self
    }

    pub fn build(self) -> Result<Salak, PropertyError> {
        let mut env = PropertyRegistry::new();

        #[cfg(feature = "rand")]
        {
            env.register_by_ref(crate::source_rand::Random);
        }

        env = env
            .register(MapProvider::new("Arguments").extend(self.args))
            .register(system_environment());

        #[cfg(any(feature = "toml", feature = "yaml"))]
        {
            let fc = FileConfig::new(&env)?;
            #[cfg(feature = "toml")]
            {
                crate::source_toml::init_toml(&mut env, &fc)?;
            }

            #[cfg(feature = "yaml")]
            {
                crate::source_yaml::init_yaml(&mut env, &fc)?;
            }
        }

        Ok(Salak(env))
    }
}

pub struct Salak(PropertyRegistry);

impl Salak {
    pub fn builder() -> SalakBuilder {
        SalakBuilder {
            args: HashMap::new(),
        }
    }

    pub fn new() -> Result<Self, PropertyError> {
        Self::builder().build()
    }

    pub fn register<P: PropertySource + 'static>(&mut self, provider: P) {
        self.0.register_by_ref(provider)
    }
}

impl Environment for Salak {
    fn require_def<T: FromEnvironment>(
        &self,
        key: &str,
        def: Option<Property<'_>>,
    ) -> Result<T, PropertyError> {
        self.0.require_def(key, def)
    }
}

impl Environment for PropertyRegistry {
    fn require_def<T: FromEnvironment>(
        &self,
        key: &str,
        def: Option<Property<'_>>,
    ) -> Result<T, PropertyError> {
        T::from_env(key, self.get(key, def)?, self)
    }
}

impl<T: FromEnvironment> FromEnvironment for Option<T> {
    fn from_env(
        key: &str,
        val: Option<Property<'_>>,
        env: &impl Environment,
    ) -> Result<Self, PropertyError> {
        match T::from_env(key, val, env) {
            Ok(v) => Ok(Some(v)),
            Err(PropertyError::NotFound(_)) => Ok(None),
            Err(err) => Err(err),
        }
    }
}

impl<T: IsProperty> FromEnvironment for T {
    fn from_env(
        key: &str,
        val: Option<Property<'_>>,
        _: &impl Environment,
    ) -> Result<Self, PropertyError> {
        match val {
            Some(v) => Self::from_property(v),
            _ => Err(PropertyError::NotFound(normalize_key(key).to_string())),
        }
    }
}
