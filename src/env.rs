use parking_lot::Mutex;
#[cfg(feature = "app")]
use std::any::Any;
use std::collections::HashMap;

#[cfg(feature = "args")]
use crate::AppInfo;

use crate::{
    raw_ioref::IORefT, source_raw::PropertyRegistryInternal, Environment, FromEnvironment,
    PropertySource, Res,
};
#[cfg(feature = "app")]
use crate::{Resource, ResourceBuilder, ResourceRegistry};

#[allow(unused_imports)]
use crate::source_raw::FileConfig;
#[cfg(feature = "derive")]
use crate::{DescFromEnvironment, Key, KeyDesc, PrefixedFromEnvironment, SalakDescContext};

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
    pub(crate) app_desc: Vec<Box<dyn Fn(&mut Salak) -> Vec<KeyDesc>>>,
    #[cfg(feature = "args")]
    app_info: Option<AppInfo<'static>>,
    iorefs: Mutex<Vec<Box<dyn IORefT + Send>>>,
    #[cfg(feature = "app")]
    resource: ResourceRegistry,
}

#[allow(dead_code)]
pub(crate) const PREFIX: &str = "salak.app";

impl SalakBuilder {
    /// Set custom arguments properties.
    #[inline]
    pub fn set_args(mut self, args: HashMap<String, String>) -> Self {
        self.args.extend(args);
        self
    }

    /// Set custom property.
    pub fn set<K: Into<String>, V: Into<String>>(mut self, k: K, v: V) -> Self {
        self.args.insert(k.into(), v.into());
        self
    }

    #[cfg(any(feature = "toml", feature = "yaml"))]
    #[cfg_attr(docsrs, doc(cfg(any(feature = "toml", feature = "yaml"))))]
    /// Configure file source.
    pub fn configure_files(mut self, enabled: bool) -> Self {
        self.disable_file = !enabled;
        self
    }

    #[cfg(feature = "rand")]
    #[cfg_attr(docsrs, doc(cfg(feature = "rand")))]
    /// Configure random source.
    pub fn configure_random(mut self, enabled: bool) -> Self {
        self.disable_random = !enabled;
        self
    }

    #[cfg(feature = "args")]
    #[cfg_attr(docsrs, doc(cfg(feature = "args")))]
    /// Configure predefined arguments.
    pub fn configure_args(mut self, info: AppInfo<'static>) -> Self {
        self.app_info = Some(info);
        self
    }

    /// Build salak.
    #[allow(unused_mut)]
    pub fn build(mut self) -> Res<Salak> {
        #[cfg(feature = "derive")]
        let mut _desc: Vec<KeyDesc> = vec![];
        #[cfg(feature = "derive")]
        #[cfg(any(feature = "toml", feature = "yaml"))]
        {
            self.app_desc
                .insert(0, Box::new(|env| env.get_desc::<FileConfig>("")));
        }
        let mut env = self.registry;

        #[cfg(feature = "rand")]
        if !self.disable_random {
            env.register_by_ref(Box::new(crate::source_rand::Random));
        }
        let mut salak = Salak {
            reg: env,
            ior: self.iorefs,
            #[cfg(feature = "app")]
            res: self.resource,
        };

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

        salak.reg = salak
            .reg
            .register(crate::source::HashMapSource::new("Arguments").set_all(self.args))
            .register(crate::source::system_environment());

        #[cfg(any(feature = "toml", feature = "yaml"))]
        if !self.disable_file {
            let mut fc = FileConfig::new(&salak.reg, &salak.ior)?;
            #[cfg(feature = "toml")]
            {
                fc.build("toml", crate::source_toml::Toml::new)?;
            }
            #[cfg(feature = "yaml")]
            {
                fc.build("yaml", crate::source_yaml::YamlValue::new)?;
            }
            fc.register_to_env(&mut salak.reg);
        }

        #[cfg(feature = "app")]
        salak.res.initialize(&salak)?;
        Ok(salak)
    }

    #[inline]
    #[cfg(feature = "app")]
    #[cfg_attr(docsrs, doc(cfg(feature = "app")))]
    /// Register [`Resource`] with default builder.
    pub fn register_default_resource<R: Resource + Send + Sync + Any>(self) -> Res<Self> {
        self.register_resource::<R>(ResourceBuilder::default())
    }

    #[inline]
    #[cfg(feature = "app")]
    #[cfg_attr(docsrs, doc(cfg(feature = "app")))]
    /// Register [`Resource`] by [`ResourceBuilder`].
    pub fn register_resource<R: Resource + Send + Sync + Any>(
        self,
        builder: ResourceBuilder<R>,
    ) -> Res<Self> {
        let mut env = self.configure_resource_description_by_builder(&builder);
        env.resource.register(builder)?;
        Ok(env)
    }

    #[inline]
    #[cfg(feature = "app")]
    /// Configure resource description.
    #[cfg_attr(docsrs, doc(cfg(feature = "app")))]
    pub(crate) fn configure_resource_description_by_builder<R: Resource>(
        self,
        builder: &ResourceBuilder<R>,
    ) -> Self {
        self.configure_description_by_namespace::<R::Config>(builder.namespace)
    }

    #[cfg(feature = "derive")]
    #[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
    /// Configure description parsing.
    pub fn configure_description<T: PrefixedFromEnvironment + DescFromEnvironment>(self) -> Self {
        self.configure_description_by_namespace::<T>("")
    }

    #[cfg(feature = "derive")]
    #[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
    /// Configure description parsing.
    pub fn configure_description_by_namespace<T: PrefixedFromEnvironment + DescFromEnvironment>(
        mut self,
        namespace: &'static str,
    ) -> Self {
        self.app_desc
            .push(Box::new(move |env| env.get_desc::<T>(namespace)));
        self
    }
}

/// Salak is a wrapper for salak env, all functions that this crate provides will be implemented on it.
/// * Provides a group of sources that have predefined orders.
/// * Provides custom source registration.
///
#[allow(missing_debug_implementations)]
pub struct Salak {
    reg: PropertyRegistryInternal<'static>,
    ior: Mutex<Vec<Box<dyn IORefT + Send>>>,
    #[cfg(feature = "app")]
    pub(crate) res: ResourceRegistry,
}

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
            #[cfg(feature = "app")]
            resource: ResourceRegistry::new(),
        }
    }

    /// Create a new salak env.
    pub fn new() -> Res<Self> {
        Self::builder().build()
    }

    /// Register source to registry, source that register earlier that higher priority for
    /// configuration.
    pub fn register<P: PropertySource + 'static>(&mut self, provider: P) {
        self.reg.register_by_ref(Box::new(provider))
    }

    #[cfg(feature = "derive")]
    /// Get key description.
    #[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
    pub(crate) fn get_desc<T: PrefixedFromEnvironment + DescFromEnvironment>(
        &self,
        namespace: &'static str,
    ) -> Vec<KeyDesc> {
        let mut key = Key::new();
        let mut key_descs = vec![];
        let mut context = SalakDescContext::new(&mut key, &mut key_descs);
        if namespace.is_empty() {
            context.add_key_desc::<T>(T::prefix(), None, None, None);
        } else {
            context.add_key_desc::<T>(&format!("{}.{}", T::prefix(), namespace), None, None, None);
        };
        key_descs
    }
}

impl Environment for Salak {
    #[inline]
    fn reload(&self) -> Res<bool> {
        self.reg.reload(&self.ior)
    }

    #[inline]
    fn require<T: FromEnvironment>(&self, key: &str) -> Res<T> {
        self.reg.require(key, &self.ior)
    }
}
