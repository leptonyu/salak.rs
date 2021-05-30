use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Mutex;

#[cfg(feature = "args")]
use crate::AppInfo;
#[allow(unused_imports)]
use crate::Key;

use crate::raw_ioref::IORefT;
use crate::{
    source_raw::PropertyRegistryInternal, Environment, FromEnvironment, IsProperty, Property,
    PropertyError, PropertySource, SalakContext,
};

#[cfg(feature = "derive")]
use crate::{KeyDesc, PrefixedFromEnvironment, SalakDescContext};

#[allow(unused_imports)]
use crate::source_raw::FileConfig;

/// A builder which can configure for how to build a salak env.
#[allow(missing_debug_implementations)]
pub struct SalakBuilder {
    args: HashMap<String, String>,
    #[cfg(any(feature = "toml", feature = "yaml"))]
    disable_file: bool,
    #[cfg(feature = "rand")]
    disable_random: bool,
    registry: PropertyRegistryInternal<'static>,
    #[cfg(any(feature = "args", feature = "derive"))]
    app_desc: Vec<Box<dyn FnOnce(&mut Salak) -> Vec<KeyDesc>>>,
    #[cfg(feature = "args")]
    app_info: Option<AppInfo<'static>>,
    iorefs: Mutex<Vec<Box<dyn IORefT + Send>>>,
}

#[allow(dead_code)]
pub(crate) const PREFIX: &str = "salak.app";

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
    pub fn configure_files(mut self, enabled: bool) -> Self {
        self.disable_file = !enabled;
        self
    }

    #[cfg(feature = "rand")]
    #[cfg_attr(docsrs, doc(cfg(feature = "rand")))]
    /// Disable random support.
    pub fn configure_random(mut self, enabled: bool) -> Self {
        self.disable_random = !enabled;
        self
    }

    #[cfg(feature = "args")]
    #[cfg_attr(docsrs, doc(cfg(feature = "args")))]
    /// Enable arguments.
    pub fn configure_args(mut self, info: AppInfo<'static>) -> Self {
        self.app_info = Some(info);
        self
    }

    #[cfg(feature = "derive")]
    #[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
    /// Enable arguments.
    pub fn configure_description<T: PrefixedFromEnvironment>(mut self) -> Self {
        self.app_desc.push(Box::new(|env| env.get_desc::<T>()));
        self
    }

    /// Build salak env.
    #[allow(unused_mut)]
    pub fn build(mut self) -> Result<Salak, PropertyError> {
        #[cfg(feature = "derive")]
        let mut _desc: Vec<KeyDesc> = vec![];
        #[cfg(feature = "derive")]
        #[cfg(any(feature = "toml", feature = "yaml"))]
        {
            self = self.configure_description::<FileConfig>();
        }
        let mut env = self.registry;

        #[cfg(feature = "rand")]
        if !self.disable_random {
            env.register_by_ref(Box::new(crate::source_rand::Random));
        }
        let mut salak = Salak(env, self.iorefs);

        #[cfg(feature = "args")]
        if let Some(app) = self.app_info {
            self.args
                .insert(format!("{}.name", PREFIX), app.name.into());
            self.args
                .insert(format!("{}.version", PREFIX), app.version.into());

            #[cfg(feature = "derive")]
            {
                _desc.push(KeyDesc::new(
                    format!("{}.name", PREFIX),
                    "String",
                    Some(false),
                    Some(app.name),
                    None,
                ));
                _desc.push(KeyDesc::new(
                    format!("{}.version", PREFIX),
                    "String",
                    Some(false),
                    Some(app.version),
                    None,
                ));

                for x in self.app_desc {
                    _desc.extend((x)(&mut salak));
                }
            }

            self.args.extend(crate::source::from_args(_desc, app)?);
        }

        salak.0 = salak
            .0
            .register(crate::source::HashMapSource::new("Arguments").set_all(self.args))
            .register(crate::source::system_environment());

        #[cfg(any(feature = "toml", feature = "yaml"))]
        if !self.disable_file {
            let mut fc = FileConfig::new(&salak.0, &salak.1)?;
            #[cfg(feature = "toml")]
            {
                fc.build("toml", crate::source_toml::Toml::new)?;
            }
            #[cfg(feature = "yaml")]
            {
                fc.build("yaml", crate::source_yaml::YamlValue::new)?;
            }
            fc.register_to_env(&mut salak.0);
        }

        Ok(salak)
    }
}

/// Salak is a wrapper for salak env, all functions that this crate provides will be implemented on it.
/// * Provides a group of sources that have predefined orders.
/// * Provides custom source registration.
///
#[allow(missing_debug_implementations)]
pub struct Salak(
    PropertyRegistryInternal<'static>,
    Mutex<Vec<Box<dyn IORefT + Send>>>,
);

impl Salak {
    /// Create a builder for configure salak env.
    pub fn builder() -> SalakBuilder {
        SalakBuilder {
            args: HashMap::new(),
            #[cfg(any(feature = "toml", feature = "yaml"))]
            disable_file: false,
            #[cfg(feature = "rand")]
            disable_random: false,
            registry: PropertyRegistryInternal::new("registry"),
            #[cfg(any(feature = "args", feature = "derive"))]
            app_desc: vec![],
            #[cfg(feature = "args")]
            app_info: None,
            iorefs: Mutex::new(vec![]),
        }
    }

    /// Create a new salak env.
    pub fn new() -> Result<Self, PropertyError> {
        Self::builder().build()
    }

    /// Register source to registry, source that register earlier that higher priority for
    /// configuration.
    pub fn register<P: PropertySource + 'static>(&mut self, provider: P) {
        self.0.register_by_ref(Box::new(provider))
    }

    /// Get key description.
    #[cfg(feature = "derive")]
    #[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
    pub(crate) fn get_desc<T: PrefixedFromEnvironment>(&self) -> Vec<KeyDesc> {
        let mut key = Key::new();
        let mut key_descs = vec![];
        let mut context = SalakDescContext::new(&mut key, &mut key_descs);
        context.add_key_desc::<T>(T::prefix(), None, None, None);
        key_descs
    }
}

impl Environment for Salak {
    fn reload(&self) -> Result<bool, PropertyError> {
        self.0.reload(&self.1)
    }

    fn require<T: FromEnvironment>(&self, key: &str) -> Result<T, PropertyError> {
        self.0.require(key, &self.1)
    }
}

impl<T: IsProperty> FromEnvironment for T {
    fn from_env(
        val: Option<Property<'_>>,
        env: &mut SalakContext<'_>,
    ) -> Result<Self, PropertyError> {
        if let Some(v) = val {
            if !Self::is_empty(&v) {
                return Self::from_property(v);
            }
        }
        Err(PropertyError::NotFound(env.current_key().to_string()))
    }

    #[cfg(feature = "derive")]
    #[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
    fn key_desc(env: &mut SalakDescContext<'_>) {
        env.current.ignore = false;
        env.current.set_required(true);
    }
}

impl<T: FromEnvironment> FromEnvironment for Vec<T> {
    fn from_env(
        _: Option<Property<'_>>,
        env: &mut SalakContext<'_>,
    ) -> Result<Self, PropertyError> {
        let mut vs = vec![];
        if let Some(max) = env.get_sub_keys().max() {
            let mut i = 0;
            while let Some(v) = env.require_def_internal::<Option<T>, usize>(i, None)? {
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
    fn key_desc(env: &mut SalakDescContext<'_>) {
        env.current.ignore = true;
        env.current.set_required(false);
        env.add_key_desc_internal::<T, usize>(
            0,
            env.current.required,
            None,
            env.current.desc.clone(),
        );
    }
}

impl<T: FromEnvironment> FromEnvironment for HashMap<String, T> {
    fn from_env(
        _: Option<Property<'_>>,
        env: &mut SalakContext<'_>,
    ) -> Result<Self, PropertyError> {
        let mut v = HashMap::new();
        for k in env.get_sub_keys().str_keys() {
            if let Some(val) = env.require_def_internal::<Option<T>, &str>(k, None)? {
                v.insert(k.to_owned(), val);
            }
        }
        Ok(v)
    }

    #[cfg(feature = "derive")]
    #[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
    fn key_desc(env: &mut SalakDescContext<'_>) {
        env.current.set_required(false);
        env.add_key_desc::<T>("*", None, None, env.current.desc.clone());
    }
}

impl<T> FromEnvironment for HashSet<T>
where
    T: Eq + FromEnvironment + std::hash::Hash,
{
    fn from_env(
        val: Option<Property<'_>>,
        env: &mut SalakContext<'_>,
    ) -> Result<Self, PropertyError> {
        Ok(<Vec<T>>::from_env(val, env)?.into_iter().collect())
    }

    #[cfg(feature = "derive")]
    #[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
    fn key_desc(env: &mut SalakDescContext<'_>) {
        <Vec<T>>::key_desc(env);
    }
}
