use crate::source::FileConfig;
use crate::*;

/// An implementation of [`Environment`] for registering [`PropertySource`].
#[allow(missing_debug_implementations)]
pub struct SourceRegistry {
    #[allow(dead_code)]
    conf: Option<FileConfig>,
    #[cfg(feature = "enable_derive")]
    pub(crate) default: Option<MapPropertySource>,
    sources: Vec<Box<dyn PropertySource>>,
}
impl SourceRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        #[allow(unused_mut)]
        let mut sr = SourceRegistry {
            conf: None,
            #[cfg(feature = "enable_derive")]
            default: None,
            sources: vec![],
        };

        #[cfg(feature = "enable_rand")]
        sr.register_source(Box::new(crate::source::rand::Random));
        sr
    }

    /// Add default command line arguments parser.
    #[cfg(feature = "enable_clap")]
    #[cfg_attr(docsrs, doc(cfg(feature = "enable_clap")))]
    pub fn with_args(mut self, mode: SysArgsMode) -> Self {
        self.register_source(Box::new(SysArgs::new(mode).0));
        self
    }

    /// Add system environment.
    pub fn with_sys_env(mut self) -> Self {
        self.register_source(Box::new(SysEnvPropertySource::new()));
        self
    }

    #[allow(dead_code)]
    fn build_conf(&mut self) -> FileConfig {
        match &self.conf {
            Some(v) => v.clone(),
            _ => {
                let v = FileConfig::new(self);
                self.conf = Some(v.clone());
                v
            }
        }
    }

    /// Add toml support.
    #[cfg(feature = "enable_toml")]
    #[cfg_attr(docsrs, doc(cfg(feature = "enable_toml")))]
    pub fn with_toml(mut self) -> Result<Self, PropertyError> {
        let fc = self.build_conf();
        self.register_sources(fc.build(Toml)?);
        Ok(self)
    }

    /// Add yaml support.
    #[cfg(feature = "enable_yaml")]
    #[cfg_attr(docsrs, doc(cfg(feature = "enable_yaml")))]
    pub fn with_yaml(mut self) -> Result<Self, PropertyError> {
        let fc = self.build_conf();
        self.register_sources(fc.build(Yaml)?);
        Ok(self)
    }

    /// Add yaml support.
    // #[cfg(feature = "enable_yaml")]
    // #[cfg_attr(docsrs, doc(cfg(feature = "enable_yaml")))]
    // pub fn with_yaml(mut self) -> Self {
    //     let fc = self.build_conf();
    //     self.register_sources(fc.build(crate::yaml::Yaml));
    //     self
    // }

    /// Register source.
    pub fn register_source(&mut self, source: Box<dyn PropertySource>) {
        if !source.is_empty() {
            #[cfg(feature = "enable_log")]
            debug!("Load property source {}.", source.name());
            self.sources.push(source);
        }
    }

    /// Register multiple sources.
    pub fn register_sources(&mut self, sources: Vec<Box<dyn PropertySource>>) {
        for source in sources.into_iter() {
            self.register_source(source);
        }
    }
}

impl Default for SourceRegistry {
    fn default() -> Self {
        let mut sr = SourceRegistry::new();
        #[cfg(not(test))]
        #[cfg(feature = "enable_clap")]
        {
            sr = sr.with_args(SysArgsMode::Auto(auto_read_sys_args_param!()));
        }
        sr = sr.with_sys_env();
        #[cfg(feature = "enable_toml")]
        {
            sr = sr.with_toml().expect("Toml load failed");
        }
        #[cfg(feature = "enable_yaml")]
        {
            sr = sr.with_yaml().expect("Yaml load failed");
        }
        sr
    }
}

impl Environment for SourceRegistry {
    fn contains(&self, name: &str) -> bool {
        self.sources.iter().any(|a| a.contains_property(name))
    }
    fn require<T: FromEnvironment>(&self, name: &str) -> Result<T, PropertyError> {
        let mut x = None;
        if !name.is_empty() {
            for ps in self.sources.iter() {
                if let Some(v) = ps.get_property(name) {
                    x = Some(v);
                    break;
                }
            }
            #[cfg(feature = "enable_derive")]
            if x.is_none() {
                if let Some(ps) = &self.default {
                    x = ps.get_property(name);
                }
            }
        }
        T::from_env(name, x, self)
    }

    fn resolve_placeholder(&self, _: String) -> Result<Option<Property>, PropertyError> {
        Err(PropertyError::parse_failed("Placeholder not implement"))
    }
    fn find_keys(&self, prefix: &str) -> Vec<String> {
        let s: HashSet<String> = self
            .sources
            .iter()
            .flat_map(|p| p.get_keys(prefix))
            .collect();
        s.into_iter().collect()
    }
    fn reload(&mut self) -> Result<(), PropertyError> {
        for ps in self.sources.iter_mut() {
            if let Some(p) = ps.load()? {
                *ps = p;
            }
        }
        Ok(())
    }
}
