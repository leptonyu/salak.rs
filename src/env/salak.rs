//! Provide [`Environment`] implementations.
use crate::*;
#[allow(unused_imports)]
use std::collections::BTreeMap;

/// [`Salak`] builder.
#[derive(Debug)]
pub struct SalakBuilder {
    args: Option<SysArgsMode>,
    enable_placeholder: bool,
    enable_default_registry: bool,
    #[cfg(feature = "enable_derive")]
    default: BTreeMap<String, Property>,
}

impl Default for SalakBuilder {
    fn default() -> Self {
        Salak::new()
    }
}

impl SalakBuilder {
    /// Use default command line arguments parser.
    /// Please use macro [`auto_read_sys_args_param!`] to generate [`SysArgsParam`].
    #[cfg(feature = "enable_clap")]
    #[cfg_attr(docsrs, doc(cfg(feature = "enable_clap")))]
    pub fn with_default_args(mut self, param: SysArgsParam) -> Self {
        self.args = Some(SysArgsMode::Auto(param));
        self
    }

    /// Use custom command line arguments parser.
    /// Users should provide a parser to produce [`Vec<(String, Property)>`].
    pub fn with_custom_args(mut self, args: Vec<(String, Property)>) -> Self {
        self.args = Some(SysArgsMode::Custom(args));
        self
    }

    /// Disable placeholder parsing.
    pub fn disable_placeholder(mut self) -> Self {
        self.enable_placeholder = false;
        self
    }

    /// Disable register default property sources.
    /// Users should organize [`PropertySource`]s themselves.
    pub fn disable_default_registry(mut self) -> Self {
        self.enable_default_registry = false;
        self
    }

    /// Add default properties to [`Environment`]
    #[cfg(feature = "enable_derive")]
    #[cfg_attr(docsrs, doc(cfg(feature = "enable_derive")))]
    pub fn add_default<T: DefaultSourceFromEnvironment>(mut self) -> Self {
        let p = T::prefix();

        #[cfg(feature = "enable_log")]
        debug!("Register default properties with prefix {}.", p);

        for (k, v) in T::load_default() {
            self.default.insert(format!("{}.{}", p, k), v);
        }
        self
    }

    /// Build a [`Salak`] environment.
    pub fn build(self) -> Salak {
        #[allow(unused_mut)]
        let mut sr = if self.enable_default_registry {
            let mut sr = SourceRegistry::new();
            // First Layer
            if let Some(p) = self.args {
                sr.register_source(Box::new(SysArgs::new(p).0));
            }
            // Second Layer
            sr = sr.with_sys_env();
            // Third Layer
            #[cfg(feature = "enable_toml")]
            {
                sr = sr.with_toml().expect("Toml load failed");
            }
            #[cfg(feature = "enable_yaml")]
            {
                sr = sr.with_yaml().expect("Yaml load failed");
            }
            sr
        } else {
            SourceRegistry::new()
        };
        #[cfg(feature = "enable_derive")]
        {
            sr.default = Some(MapPropertySource::new("default", self.default));
        }
        Salak(FactoryRegistry::new(PlaceholderResolver::new(
            self.enable_placeholder,
            sr,
        )))
    }
}

/// A wrapper for [`Environment`], which can hide the implementation details.
#[allow(missing_debug_implementations)]
pub struct Salak(FactoryRegistry<PlaceholderResolver<SourceRegistry>>);

impl Salak {
    /// Register property source at last.
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> SalakBuilder {
        SalakBuilder {
            args: None,
            enable_placeholder: true,
            enable_default_registry: true,
            #[cfg(feature = "enable_derive")]
            default: BTreeMap::new(),
        }
    }
    fn get_registry(&mut self) -> &mut SourceRegistry {
        &mut self.0.env.env
    }

    /// Create default builder.
    pub fn register_source(&mut self, ps: Box<dyn PropertySource>) {
        self.get_registry().register_source(ps);
    }
    /// Register property sources at last.
    pub fn register_sources(&mut self, sources: Vec<Box<dyn PropertySource>>) {
        self.get_registry().register_sources(sources);
    }
}

impl Default for Salak {
    fn default() -> Self {
        Salak::new().build()
    }
}

impl Environment for Salak {
    fn contains(&self, name: &str) -> bool {
        self.0.contains(name)
    }
    fn require<T: FromEnvironment>(&self, name: &str) -> Result<T, PropertyError> {
        self.0.require(name)
    }
    fn resolve_placeholder(&self, value: String) -> Result<Option<Property>, PropertyError> {
        self.0.resolve_placeholder(value)
    }
    fn find_keys(&self, prefix: &str) -> Vec<String> {
        self.0.find_keys(prefix)
    }
    fn reload(&mut self) -> Result<(), PropertyError> {
        self.0.reload()
    }
}

impl Factory for Salak {
    type Env = Salak;
    fn get_env(&self) -> &Self::Env {
        self
    }
    fn fetch<T: FromFactory>(&self) -> Result<FacRef<T>, PropertyError> {
        self.0.fetch()
    }
}
