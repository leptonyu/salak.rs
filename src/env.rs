use std::collections::HashMap;

use std::collections::HashSet;

use crate::{
    source::{system_environment, FileConfig, HashMapSource, PropertyRegistry},
    Environment, FromEnvironment, IsProperty, Key, Property, PropertyError, PropertySource, SubKey,
    SubKeys,
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

    /// Build salak env, and panic if any error happens.
    pub fn unwrap_build(self) -> Salak {
        self.build().unwrap()
    }

    /// Build salak env.
    pub fn build(self) -> Result<Salak, PropertyError> {
        let mut env = PropertyRegistry::new();

        #[cfg(feature = "rand")]
        {
            env.register_by_ref(crate::source_rand::Random);
        }

        env = env
            .register(HashMapSource::new("Arguments").set_all(self.args))
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
/// * Provides a group of sources that have predefined orders.
/// * Provides custom source registration.
///
/// Predefined sources:
/// 1. Source for generating random values with following keys..
///    * random.i8
///    * random.i16
///    * random.i32
///    * random.i64
///    * random.u8
///    * random.u16
///    * random.u32
/// 2. Source from arguments and direct coding.
/// 3. Source from environment.
/// 4. Source from toml if feature enabled.
/// 5. Source from yaml if feature enabled.
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
    fn sub_keys<'a>(&'a self, prefix: &Key<'_>, sub_keys: &mut SubKeys<'a>) {
        PropertySource::sub_keys(&self.0, prefix, sub_keys)
    }

    fn require_def<'a, T: FromEnvironment, K: Into<SubKey<'a>>>(
        &'a self,
        key: &mut Key<'a>,
        sub_key: K,
        def: Option<Property<'_>>,
    ) -> Result<T, PropertyError> {
        self.0.require_def(key, sub_key, def)
    }
}

impl Environment for PropertyRegistry {
    fn require_def<'a, T: FromEnvironment, K: Into<SubKey<'a>>>(
        &'a self,
        key: &mut Key<'a>,
        sub_key: K,
        def: Option<Property<'_>>,
    ) -> Result<T, PropertyError> {
        key.push(sub_key.into());
        let val = self.get(key, def).map(|val| T::from_env(key, val, self));
        key.pop();
        val?
    }

    fn sub_keys<'a>(&'a self, prefix: &Key<'_>, sub_keys: &mut SubKeys<'a>) {
        PropertySource::sub_keys(self, prefix, sub_keys)
    }
}

impl<T: FromEnvironment> FromEnvironment for Option<T> {
    fn from_env<'a>(
        key: &mut Key<'a>,
        val: Option<Property<'_>>,
        env: &'a impl Environment,
    ) -> Result<Self, PropertyError> {
        match T::from_env(key, val, env) {
            Ok(v) => Ok(Some(v)),
            Err(PropertyError::NotFound(_)) => Ok(None),
            Err(err) => Err(err),
        }
    }
}

impl<T: IsProperty> FromEnvironment for T {
    fn from_env<'a>(
        key: &mut Key<'a>,
        val: Option<Property<'_>>,
        _: &'a impl Environment,
    ) -> Result<Self, PropertyError> {
        match val {
            Some(v) => Self::from_property(v),
            _ => Err(PropertyError::NotFound(key.as_str().to_string())),
        }
    }
}

impl<T: FromEnvironment> FromEnvironment for Vec<T> {
    fn from_env<'a>(
        key: &mut Key<'a>,
        _: Option<Property<'_>>,
        env: &'a impl Environment,
    ) -> Result<Self, PropertyError> {
        let mut sub_keys = SubKeys::new();
        env.sub_keys(key, &mut sub_keys);
        let mut vs = vec![];
        if let Some(max) = sub_keys.upper {
            let mut i = 0;
            while let Some(v) = env.require_def::<Option<T>, usize>(key, i, None)? {
                vs.push(v);
                i += 1;
                if i > max {
                    break;
                }
            }
        }
        Ok(vs)
    }
}

impl<T: FromEnvironment> FromEnvironment for HashMap<String, T> {
    fn from_env<'a>(
        key: &mut Key<'a>,
        _: Option<Property<'_>>,
        env: &'a impl Environment,
    ) -> Result<Self, PropertyError> {
        let mut sub_keys = SubKeys::new();
        env.sub_keys(key, &mut sub_keys);
        let mut v = HashMap::new();
        for k in sub_keys.str_keys() {
            if let Some(val) = env.require_def::<Option<T>, &str>(key, k, None)? {
                v.insert(k.to_owned(), val);
            }
        }
        Ok(v)
    }
}

impl<T> FromEnvironment for HashSet<T>
where
    T: Eq + FromEnvironment + std::hash::Hash,
{
    fn from_env<'a>(
        key: &mut Key<'a>,
        val: Option<Property<'_>>,
        env: &'a impl Environment,
    ) -> Result<Self, PropertyError> {
        Ok(<Vec<T>>::from_env(key, val, env)?.into_iter().collect())
    }
}

impl<'a> SubKeys<'a> {
    pub(crate) fn str_keys(&self) -> Vec<&'a str> {
        self.keys
            .iter()
            .filter(|a| {
                if let Some(c) = a.chars().next() {
                    c < '0' && c > '9'
                } else {
                    false
                }
            })
            .copied()
            .collect()
    }

    pub(crate) fn new() -> Self {
        Self {
            keys: HashSet::new(),
            upper: None,
        }
    }
}
