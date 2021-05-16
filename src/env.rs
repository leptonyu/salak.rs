use std::{collections::HashMap, ops::DerefMut};

use std::collections::HashSet;

use crate::{
    source::{system_environment, HashMapSource, PropertyRegistry},
    Environment, FromEnvironment, IsProperty, Key, Property,
    PropertyError, PropertySource, SubKey, SubKeys,
};

#[cfg(feature = "derive")]
use crate::KeyDesc;

#[cfg(feature = "args")]
use crate::{from_args, AppInfo};

#[allow(unused_imports)]
use crate::source::FileConfig;

/// A builder which can configure for how to build a salak env.
#[allow(missing_debug_implementations)]
pub struct SalakBuilder {
    args: HashMap<String, String>,
    disable_file: bool,
    disable_random: bool,
    registry: PropertyRegistry,
    #[cfg(feature = "args")]
    app_desc: Vec<KeyDesc>,
    #[cfg(feature = "args")]
    app_info: Option<AppInfo<'static>>,
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

    #[cfg(any(feature = "toml", feature = "yaml"))]
    #[cfg_attr(docsrs, doc(cfg(any(feature = "toml", feature = "yaml"))))]
    /// Not add file source to environment.
    pub fn disable_load_file(mut self) -> Self {
        self.disable_file = true;
        self
    }

    #[cfg(feature = "rand")]
    #[cfg_attr(docsrs, doc(cfg(feature = "rand")))]
    /// Disable random support.
    pub fn disable_random(mut self) -> Self {
        self.disable_random = true;
        self
    }

    #[cfg(feature = "args")]
    #[cfg_attr(docsrs, doc(cfg(feature = "args")))]
    /// Enable arguments.
    pub fn enable_args(mut self, info: AppInfo<'static>) -> Self {
        self.app_info = Some(info);
        self
    }

    #[cfg(feature = "args")]
    #[cfg_attr(docsrs, doc(cfg(feature = "args")))]
    /// Enable arguments.
    pub fn add_config_desc<T: PrefixedFromEnvironment>(mut self) -> Self {
        self.app_desc.extend(self.registry.get_desc::<T>());
        self
    }

    /// Build salak env, and panic if any error happens.
    pub fn unwrap_build(self) -> Salak {
        self.build().unwrap()
    }

    /// Build salak env.
    #[allow(unused_mut)]
    pub fn build(mut self) -> Result<Salak, PropertyError> {
        let mut env = self.registry;

        #[cfg(feature = "rand")]
        if !self.disable_random {
            env.register_by_ref(crate::source_rand::Random);
        }

        #[cfg(feature = "args")]
        if let Some(app) = self.app_info {
            self.args.extend(from_args(self.app_desc, app)?);
        }

        env = env
            .register(HashMapSource::new("Arguments").set_all(self.args))
            .register(system_environment());

        #[cfg(any(feature = "toml", feature = "yaml"))]
        if !self.disable_file {
            let mut fc = FileConfig::new(&env)?;
            #[cfg(feature = "toml")]
            {
                fc.build("toml", crate::source_toml::Toml::new)?;
            }
            #[cfg(feature = "yaml")]
            {
                fc.build("yaml", crate::source_yaml::YamlValue::new)?;
            }
            fc.register_to_env(&mut env);
        }

        Ok(Salak(env))
    }
}

/// Salak is a wrapper for salak env, all functions that this crate provides will be implemented on it.
/// * Provides a group of sources that have predefined orders.
/// * Provides custom source registration.
///
#[allow(missing_debug_implementations)]
pub struct Salak(PropertyRegistry);

impl Salak {
    /// Create a builder for configure salak env.
    pub fn builder() -> SalakBuilder {
        SalakBuilder {
            args: HashMap::new(),
            disable_file: false,
            disable_random: false,
            registry: PropertyRegistry::new("registry"),
            #[cfg(feature = "args")]
            app_desc: vec![],
            #[cfg(feature = "args")]
            app_info: None,
        }
    }

    /// Create a new salak env.
    pub fn new() -> Result<Self, PropertyError> {
        Self::builder().build()
    }

    /// Register source to registry, source that register earlier that higher priority for
    /// configuration.
    pub fn register<P: PropertySource + Send + 'static>(&mut self, provider: P) {
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

    #[cfg(feature = "derive")]
    #[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
    fn key_desc<'a, T: FromEnvironment, K: Into<SubKey<'a>>>(
        &'a self,
        key: &mut Key<'a>,
        sub_key: K,
        required: Option<bool>,
        def: Option<&'a str>,
        desc: Option<String>,
        keys: &mut Vec<KeyDesc>,
    ) {
        self.0
            .key_desc::<T, K>(key, sub_key, required, def, desc, keys);
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
        match val? {
            Err(PropertyError::ParseFail(None, v)) if !key.as_str().is_empty() => {
                Err(PropertyError::ParseFail(Some(key.as_str().to_string()), v))
            }
            val => val,
        }
    }

    fn sub_keys<'a>(&'a self, prefix: &Key<'_>, sub_keys: &mut SubKeys<'a>) {
        PropertySource::sub_keys(self, prefix, sub_keys)
    }

    #[cfg(feature = "derive")]
    #[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
    fn key_desc<'a, T: FromEnvironment, K: Into<SubKey<'a>>>(
        &'a self,
        key: &mut Key<'a>,
        sub_key: K,
        required: Option<bool>,
        def: Option<&'a str>,
        desc: Option<String>,
        keys: &mut Vec<KeyDesc>,
    ) {
        key.push(sub_key.into());
        let mut desc = KeyDesc::new(key.as_generic(), required, def, desc);
        T::key_desc(key, &mut desc, keys, self);
        if !desc.ignore {
            keys.push(desc);
        }
        key.pop();
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

    #[cfg(feature = "derive")]
    #[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
    fn key_desc<'a>(
        key: &mut Key<'a>,
        desc: &mut KeyDesc,
        keys: &mut Vec<KeyDesc>,
        env: &'a impl Environment,
    ) {
        desc.set_required(false);
        T::key_desc(key, desc, keys, env);
    }
}

impl<T: IsProperty> FromEnvironment for T {
    fn from_env<'a>(
        key: &mut Key<'a>,
        val: Option<Property<'_>>,
        _: &'a impl Environment,
    ) -> Result<Self, PropertyError> {
        if let Some(v) = val {
            if !Self::is_empty(&v) {
                return Self::from_property(v);
            }
        }
        Err(PropertyError::NotFound(key.as_str().to_string()))
    }

    #[cfg(feature = "derive")]
    #[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
    fn key_desc<'a>(
        _: &mut Key<'a>,
        desc: &mut KeyDesc,
        _: &mut Vec<KeyDesc>,
        _: &'a impl Environment,
    ) {
        desc.ignore = false;
        desc.set_required(true);
    }
}

/// A wrapper of [`Vec<T>`], but require having at least one value when parsing configuration.
#[derive(Debug)]
pub struct NonEmptyVec<T>(pub Vec<T>);

impl<T> std::ops::Deref for NonEmptyVec<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for NonEmptyVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: FromEnvironment> FromEnvironment for NonEmptyVec<T> {
    fn from_env<'a>(
        key: &mut Key<'a>,
        val: Option<Property<'_>>,
        env: &'a impl Environment,
    ) -> Result<Self, PropertyError> {
        let v = <Vec<T>>::from_env(key, val, env)?;
        if v.is_empty() {
            return Err(PropertyError::NotFound(key.as_str().to_string()));
        }
        Ok(NonEmptyVec(v))
    }

    #[cfg(feature = "derive")]
    #[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
    fn key_desc<'a>(
        key: &mut Key<'a>,
        desc: &mut KeyDesc,
        keys: &mut Vec<KeyDesc>,
        env: &'a impl Environment,
    ) {
        desc.set_required(true);
        <Vec<T>>::key_desc(key, desc, keys, env);
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

    #[cfg(feature = "derive")]
    #[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
    fn key_desc<'a>(
        key: &mut Key<'a>,
        desc: &mut KeyDesc,
        keys: &mut Vec<KeyDesc>,
        env: &'a impl Environment,
    ) {
        desc.ignore = true;
        desc.set_required(false);
        env.key_desc::<T, usize>(key, 0, desc.required, None, desc.desc.clone(), keys);
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

    #[cfg(feature = "derive")]
    #[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
    fn key_desc<'a>(
        key: &mut Key<'a>,
        desc: &mut KeyDesc,
        keys: &mut Vec<KeyDesc>,
        env: &'a impl Environment,
    ) {
        desc.set_required(false);
        env.key_desc::<T, &str>(key, "*", None, None, desc.desc.clone(), keys);
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

    #[cfg(feature = "derive")]
    #[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
    fn key_desc<'a>(
        key: &mut Key<'a>,
        desc: &mut KeyDesc,
        keys: &mut Vec<KeyDesc>,
        env: &'a impl Environment,
    ) {
        <Vec<T>>::key_desc(key, desc, keys, env);
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
