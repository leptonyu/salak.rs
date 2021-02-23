//! Provide [`Environment`] implementations.
use crate::*;

/// [`Salak`] builder.
#[derive(Debug)]
pub struct SalakBuilder {
    args: Option<SysArgsMode>,
    enable_placeholder: bool,
    enable_default_registry: bool,
    #[cfg(feature = "enable_derive")]
    default: MapPropertySource,
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
    pub fn with_custom_args<P: IntoProperty>(mut self, args: Vec<(String, P)>) -> Self {
        self.args = Some(SysArgsMode::Custom(
            args.into_iter()
                .map(|(k, v)| (k, v.into_property()))
                .collect(),
        ));
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
                sr = sr.with_toml();
            }
            #[cfg(feature = "enable_yaml")]
            {
                sr = sr.with_yaml();
            }
            sr
        } else {
            SourceRegistry::new()
        };
        #[cfg(feature = "enable_derive")]
        {
            sr.default = Some(self.default);
        }
        Salak(PlaceholderResolver::new(self.enable_placeholder, sr))
    }
}

/// A wrapper for [`Environment`], which can hide the implementation details.
#[allow(missing_debug_implementations)]
pub struct Salak(PlaceholderResolver<SourceRegistry>);

impl Salak {
    /// Register property source at last.
    pub fn new() -> SalakBuilder {
        SalakBuilder {
            args: None,
            enable_placeholder: true,
            enable_default_registry: true,
            #[cfg(feature = "enable_derive")]
            default: MapPropertySource::empty("default"),
        }
    }
    /// Create default builder.
    pub fn register_source(&mut self, ps: Box<dyn PropertySource>) {
        self.0.env.register_source(ps);
    }
    /// Register property sources at last.
    pub fn register_sources(&mut self, sources: Vec<Box<dyn PropertySource>>) {
        self.0.env.register_sources(sources);
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
    fn require<T>(&self, name: &str) -> Result<T, PropertyError>
    where
        T: FromEnvironment,
    {
        self.0.require(name)
    }
    fn resolve_placeholder(&self, value: String) -> Result<Option<Property>, PropertyError> {
        self.0.resolve_placeholder(value)
    }
}

impl<P: FromProperty> FromEnvironment for P {
    fn from_env(
        n: &str,
        property: Option<Property>,
        _: &impl Environment,
    ) -> Result<Self, PropertyError> {
        if let Some(p) = property {
            return P::from_property(p);
        }
        P::from_err(PropertyError::NotFound(n.to_owned()))
    }
}

impl<P: FromEnvironment> FromEnvironment for Option<P> {
    fn from_env(
        n: &str,
        property: Option<Property>,
        env: &impl Environment,
    ) -> Result<Self, PropertyError> {
        match P::from_env(n, property, env) {
            Ok(a) => Ok(Some(a)),
            Err(err) => Self::from_err(err),
        }
    }
    fn from_err(err: PropertyError) -> Result<Self, PropertyError> {
        match err {
            PropertyError::NotFound(_) => Ok(None),
            _ => Err(err),
        }
    }
    fn check_is_empty(&self) -> bool {
        self.is_none()
    }

    fn load_default() -> Vec<(String, Property)> {
        P::load_default()
    }
}

impl<P: FromEnvironment> FromEnvironment for Vec<P> {
    fn from_env(
        name: &str,
        _: Option<Property>,
        env: &impl Environment,
    ) -> Result<Self, PropertyError> {
        let mut vs = vec![];
        let mut i = 0;
        let mut key = format!("{}[{}]", &name, i);
        while let Some(v) =
            <Option<P>>::from_env(&key, env.require::<Option<Property>>(&key)?, env)?
        {
            if v.check_is_empty() {
                break;
            }
            vs.push(v);
            i += 1;
            key = format!("{}[{}]", &name, i);
        }
        Ok(vs)
    }
    fn check_is_empty(&self) -> bool {
        self.is_empty()
    }
}

impl<T, S> FromEnvironment for HashSet<T, S>
where
    T: Eq + Hash + FromEnvironment,
    S: BuildHasher + Default,
{
    fn from_env(
        name: &str,
        p: Option<Property>,
        env: &impl Environment,
    ) -> Result<Self, PropertyError> {
        Ok(<Vec<T>>::from_env(name, p, env)?.into_iter().collect())
    }
}
