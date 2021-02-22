//! Provide [`Environment`] implementations.
use crate::file::FileConfig;
use crate::*;

/// An implementation of [`Environment`] that can resolve placeholder for values.
///
/// ```
/// use salak::*;
/// std::env::set_var("v1", "value");
/// std::env::set_var("v2", "${v1}");
/// std::env::set_var("v3", "${no_found:default}");
/// std::env::set_var("v4", "${no_found:${v2}}");
/// let env = PlaceholderResolver::new(true, SourceRegistry::default());
/// assert_eq!("value", &env.require::<String>("v1").unwrap());
/// assert_eq!("value", &env.require::<String>("v2").unwrap());
/// assert_eq!("default", &env.require::<String>("v3").unwrap());
/// assert_eq!("value", &env.require::<String>("v4").unwrap());
/// ```
#[derive(Debug)]
pub struct PlaceholderResolver<T: Environment> {
    enabled: bool,
    pub(crate) env: T,
    placeholder_prefix: char,
    placeholder_suffix: char,
    placeholder_middle: char,
}

impl<E: Environment> PlaceholderResolver<E> {
    /// Create placeholder environment.
    pub fn new(enabled: bool, env: E) -> Self {
        PlaceholderResolver {
            enabled,
            env,
            placeholder_prefix: '{',
            placeholder_suffix: '}',
            placeholder_middle: ':',
        }
    }

    fn require_with_parse<T: FromEnvironment>(
        &self,
        name: &str,
        contains: &mut HashSet<String>,
    ) -> Result<T, PropertyError> {
        if !contains.insert(name.to_owned()) {
            return Err(PropertyError::RecursiveParse(name.to_owned()));
        }
        let p = match self.env.require::<Option<Property>>(name)? {
            Some(Property::Str(s)) => self.parse_value(&s, contains)?,
            v => v,
        };
        T::from_env(name, p, self)
    }

    fn parse_value(
        &self,
        mut val: &str,
        contains: &mut HashSet<String>,
    ) -> Result<Option<Property>, PropertyError> {
        let mut stack: Vec<String> = vec![];
        let mut pre = "".to_owned();
        let placeholder: &[_] = &['$', '\\', self.placeholder_suffix];
        let prefix = &self.placeholder_prefix.to_string();
        while let Some(left) = val.find(placeholder) {
            match &val[left..=left] {
                "$" => {
                    let (push, next) =
                        if val.len() == left + 1 || &val[left + 1..=left + 1] != prefix {
                            (&val[..=left], &val[left + 1..])
                        } else {
                            (&val[..left], &val[left + 2..])
                        };
                    if stack.is_empty() {
                        pre.push_str(push);
                        stack.push("".to_owned());
                    } else {
                        stack.push(push.to_string());
                    }
                    val = next;
                }
                "\\" => {
                    if val.len() == left + 1 {
                        return Err(PropertyError::parse_failed("End with single \\"));
                    }
                    let merge = format!("{}{}", &val[..left], &val[left + 1..=left + 1]);
                    if let Some(mut v) = stack.pop() {
                        v.push_str(&merge);
                        stack.push(v);
                    } else {
                        pre.push_str(&merge);
                    }
                    val = &val[left + 2..];
                }
                _ => {
                    if let Some(mut name) = stack.pop() {
                        name.push_str(&val[..left]);
                        let mut def: Option<String> = None;
                        let key = if let Some(k) = name.find(self.placeholder_middle) {
                            def = Some(name[k + 1..].to_owned());
                            &name[..k]
                        } else {
                            &name
                        };
                        let value = if let Some(d) = def {
                            self.require_with_parse::<Option<String>>(&key, contains)?
                                .unwrap_or(d)
                        } else {
                            self.require_with_parse::<String>(&key, contains)?
                        };
                        if let Some(mut prefix) = stack.pop() {
                            prefix.push_str(&value);
                            stack.push(prefix);
                        } else {
                            pre.push_str(&value);
                        }
                    } else {
                        return Err(PropertyError::parse_failed("Suffix not match 1"));
                    }
                    val = &val[left + 1..];
                }
            }
        }
        if !stack.is_empty() {
            return Err(PropertyError::parse_failed("Suffix not match 2"));
        }
        pre.push_str(&val);
        Ok(Some(Property::Str(pre)))
    }
}

impl<E: Environment> Environment for PlaceholderResolver<E> {
    fn contains(&self, name: &str) -> bool {
        self.env.contains(name)
    }

    fn require<T>(&self, name: &str) -> Result<T, PropertyError>
    where
        T: FromEnvironment,
    {
        if self.enabled && !name.is_empty() {
            self.require_with_parse::<T>(name, &mut HashSet::new())
        } else {
            self.env.require(name)
        }
    }

    fn resolve_placeholder(&self, value: String) -> Result<Option<Property>, PropertyError> {
        self.parse_value(&value, &mut HashSet::new())
    }
}

/// An implementation of [`Environment`] for registering [`PropertySource`].
#[allow(missing_debug_implementations)]
pub struct SourceRegistry {
    #[allow(dead_code)]
    conf: Option<FileConfig>,
    #[cfg(feature = "enable_derive")]
    default: std::sync::RwLock<(HashSet<String>, MapPropertySource)>,
    sources: Vec<Box<dyn PropertySource>>,
}

impl SourceRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        SourceRegistry {
            conf: None,
            #[cfg(feature = "enable_derive")]
            default: std::sync::RwLock::new((HashSet::new(), MapPropertySource::empty("default"))),
            sources: vec![],
        }
    }

    /// Add default command line arguments parser.
    #[cfg(feature = "enable_clap")]
    #[cfg_attr(docsrs, doc(cfg(feature = "enable_clap")))]
    pub fn with_args(mut self, mode: SysArgsMode) -> Self {
        self.register_source(Box::new(args::SysArgs::new(mode).0));
        self
    }

    /// Add system environment.
    pub fn with_sys_env(mut self) -> Self {
        self.register_source(Box::new(env::SysEnv));
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
    pub fn with_toml(mut self) -> Self {
        let fc = self.build_conf();
        self.register_sources(fc.build(crate::toml::Toml));
        self
    }

    /// Add yaml support.
    #[cfg(feature = "enable_yaml")]
    #[cfg_attr(docsrs, doc(cfg(feature = "enable_yaml")))]
    pub fn with_yaml(mut self) -> Self {
        let fc = self.build_conf();
        self.register_sources(fc.build(crate::yaml::Yaml));
        self
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
            sr = sr.with_args(args::SysArgsMode::Auto(auto_read_sys_args_param!()));
        }
        sr = sr.with_sys_env();
        #[cfg(feature = "enable_toml")]
        {
            sr = sr.with_toml();
        }
        #[cfg(feature = "enable_yaml")]
        {
            sr = sr.with_yaml();
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
            {
                x = x.or_else(|| {
                    self.default
                        .read()
                        .expect("READ lock failed")
                        .1
                        .get_property(name)
                });
            }
        }
        T::from_env(name, x, self)
    }

    fn resolve_placeholder(&self, _: String) -> Result<Option<Property>, PropertyError> {
        Err(PropertyError::parse_failed("Placeholder not implement"))
    }
}

#[cfg(test)]
mod tests {

    use crate::environment::*;

    #[test]
    fn check() {
        std::env::set_var("v1", "value");
        std::env::set_var("v2", "${v1}");
        std::env::set_var("v3", "${no_found:default}");
        std::env::set_var("v4", "${no_found:${v2}}");
        std::env::set_var("v5", "${no_found:${no_found_2:hello}}");
        std::env::set_var("v6", "hello-${v1}-${v3}-");
        std::env::set_var("v7", "${v7}");
        std::env::set_var("v10", "${no_found}");
        std::env::set_var("v11", "\\{raw\\}");
        let env = PlaceholderResolver::new(true, SourceRegistry::default());
        assert_eq!("value", &env.require::<String>("v1").unwrap());
        assert_eq!("value", &env.require::<String>("v2").unwrap());
        assert_eq!("default", &env.require::<String>("v3").unwrap());
        assert_eq!("value", &env.require::<String>("v4").unwrap());
        assert_eq!("hello", &env.require::<String>("v5").unwrap());
        assert_eq!(
            "hello-value-default-",
            &env.require::<String>("v6").unwrap()
        );

        let v7 = env.require::<String>("v7");

        assert_eq!(true, v7.is_err());
        assert_eq!(
            PropertyError::RecursiveParse("v7".to_string()),
            v7.unwrap_err()
        );

        let v8 = env.require::<Option<String>>("v8");
        assert_eq!(true, v8.is_ok());
        let v9 = env.require::<Option<String>>("");
        assert_eq!(true, v9.is_ok());
        assert_eq!(None, v9.unwrap());

        let v10 = env.require::<String>("v10");
        assert_eq!(true, v10.is_err());
        assert_eq!(
            PropertyError::NotFound("no_found".to_owned()),
            v10.unwrap_err()
        );
        assert_eq!(Ok("{raw}".to_owned()), env.require::<String>("v11"));
    }
}

/// [`Salak`] builder.
#[derive(Debug)]
pub struct SalakBuilder {
    args: Option<SysArgsMode>,
    enable_placeholder: bool,
    enable_default_registry: bool,
}

impl Default for SalakBuilder {
    fn default() -> Self {
        Salak::new()
    }
}

impl SalakBuilder {
    /// Use default command line arguments parser.
    /// Please use macro [`auto_read_sys_args_param!`] to generate [`args::SysArgsParam`].
    #[cfg(feature = "enable_clap")]
    #[cfg_attr(docsrs, doc(cfg(feature = "enable_clap")))]
    pub fn with_default_args(mut self, param: SysArgsParam) -> Self {
        self.args = Some(args::SysArgsMode::Auto(param));
        self
    }

    /// Use custom command line arguments parser.
    /// Users should provide a parser to produce [`Vec<(String, Property)>`].
    pub fn with_custom_args<P: IntoProperty>(mut self, args: Vec<(String, P)>) -> Self {
        self.args = Some(args::SysArgsMode::Custom(
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

    /// Build a [`Salak`] environment.
    pub fn build(self) -> Salak {
        let sr = if self.enable_default_registry {
            let mut sr = SourceRegistry::new();
            // First Layer
            if let Some(p) = self.args {
                sr.register_source(Box::new(args::SysArgs::new(p).0));
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
