use std::collections::HashMap;

use crate::{
    normalize_key,
    source::{system_environment, FileConfig, MapProvider, PropertyRegistry},
    Environment, FromEnvironment, IsProperty, Property, PropertyError, PropertySource,
};

/// A builder which can configure for how to build a salak env.
#[derive(Debug)]
pub struct SalakBuilder {
    args: HashMap<String, String>,
}

impl SalakBuilder {
    /// Set argument properties.
    pub fn set_args(mut self, args: HashMap<String, String>) -> Self {
        self.args.extend(args);
        self
    }

    /// Set property by coding.
    pub fn set<K: Into<String>, V: Into<String>>(mut self, k: K, v: V) -> Self {
        self.args.insert(k.into(), v.into());
        self
    }

    /// Build salak env.
    pub fn build(self) -> Result<Salak, PropertyError> {
        let mut env = PropertyRegistry::new();

        #[cfg(feature = "rand")]
        {
            env.register_by_ref(crate::source_rand::Random);
        }

        env = env
            .register(MapProvider::new("Arguments").set_all(self.args))
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

/// Salak is a wrapper for salak env, all functions this crate provides will be implemented on it.
/// > Provides a group of sources that have predefined orders.
/// > Provides custom source registration.
///
/// Predefined sources:
/// 0. Source for generating random values with following keys..
///   > random.i8
///   > random.i16
///   > random.i32
///   > random.i64
///   > random.u8
///   > random.u16
///   > random.u32
/// 1. Source from arguments and direct coding.
/// 2. Source from environment.
/// 3. Source from toml if feature enabled.
/// 4. Source from yaml if feature enabled.
#[allow(missing_debug_implementations)]
pub struct Salak(PropertyRegistry);

impl Salak {
    /// Create a builder for configure salak env.
    pub fn builder() -> SalakBuilder {
        SalakBuilder {
            args: HashMap::new(),
        }
    }

    /// Create a new salak env.
    pub fn new() -> Result<Self, PropertyError> {
        Self::builder().build()
    }

    /// Register source to registry, source that register earlier that higher priority for
    /// configuration.
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

impl<T: FromEnvironment> FromEnvironment for Vec<T> {
    fn from_env(
        key: &str,
        _: Option<Property<'_>>,
        env: &impl Environment,
    ) -> Result<Self, PropertyError> {
        let mut vs = vec![];
        let mut i = 0;
        while let Some(v) = env.require::<Option<T>>(&format!("{}[{}]", key, i))? {
            vs.push(v);
            i += 1;
        }
        Ok(vs)
    }
}
