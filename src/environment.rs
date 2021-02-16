//! Provide `Environment` implementations.
#[cfg(feature = "enable_toml")]
use crate::toml::Toml;
use crate::*;

/// `Environment` that can resolve placeholder for values.
///
/// ```
/// use salak::environment::*;
/// use salak::Environment;
/// std::env::set_var("v1", "value");
/// std::env::set_var("v2", "{v1}");
/// std::env::set_var("v3", "{no_found:default}");
/// std::env::set_var("v4", "{no_found:{v2}}");
/// let env = PlaceHolderEnvironment::new(true, SourceRegistry::default());
/// assert_eq!("value", &env.require::<String>("v1").unwrap());
/// assert_eq!("value", &env.require::<String>("v2").unwrap());
/// assert_eq!("default", &env.require::<String>("v3").unwrap());
/// assert_eq!("value", &env.require::<String>("v4").unwrap());
/// ```
pub struct PlaceHolderEnvironment<T: Environment> {
    enabled: bool,
    pub(crate) env: T,
    placeholder_prefix: char,
    placeholder_suffix: char,
    placeholder_middle: char,
}

impl<E: Environment> PlaceHolderEnvironment<E> {
    pub fn new(enabled: bool, env: E) -> Self {
        PlaceHolderEnvironment {
            enabled,
            env,
            placeholder_prefix: '{',
            placeholder_suffix: '}',
            placeholder_middle: ':',
        }
    }

    fn do_parse<T: FromProperty>(
        &self,
        name: &str,
        contains: &mut HashSet<String>,
    ) -> Result<T, PropertyError> {
        if !contains.insert(name.to_owned()) {
            return Err(PropertyError::RecursiveParse(name.to_owned()));
        }
        match self.env.require(name)? {
            Property::Str(s) => T::from_property(Property::Str(self.parse(&s, contains)?)),
            p => T::from_property(p),
        }
    }

    fn parse(
        &self,
        mut val: &str,
        contains: &mut HashSet<String>,
    ) -> Result<String, PropertyError> {
        let mut stack: Vec<String> = vec![];
        let mut pre = "".to_owned();
        let placeholder: &[_] = &[self.placeholder_prefix, self.placeholder_suffix];
        let prefix = &self.placeholder_prefix.to_string();
        while let Some(left) = val.find(placeholder) {
            if &val[left..=left] == prefix {
                if stack.is_empty() {
                    pre.push_str(&val[..left]);
                    stack.push("".to_owned());
                } else {
                    stack.push(val[..left].to_string());
                }
            } else {
                if let Some(mut name) = stack.pop() {
                    name.push_str(&val[..left]);
                    let mut def: Option<String> = None;
                    let key = if let Some(k) = name.find(self.placeholder_middle) {
                        def = Some(name[k + 1..].to_owned());
                        &name[..k]
                    } else {
                        &name
                    };
                    let value: String = self.do_parse(&key, contains).or_else(|e| def.ok_or(e))?;
                    if let Some(mut prefix) = stack.pop() {
                        prefix.push_str(&value);
                        stack.push(prefix);
                    } else {
                        pre.push_str(&value);
                    }
                } else {
                    return Err(PropertyError::ParseFail(format!("Suffix not match 1")));
                }
            }
            val = &val[left + 1..];
        }
        if !stack.is_empty() {
            return Err(PropertyError::ParseFail(format!("Suffix not match 2")));
        }
        pre.push_str(&val);
        Ok(pre)
    }
}

impl<E: Environment> Environment for PlaceHolderEnvironment<E> {
    fn contains(&self, name: &str) -> bool {
        self.env.contains(name)
    }

    fn require<T>(&self, name: &str) -> Result<T, PropertyError>
    where
        T: FromProperty,
    {
        if self.enabled {
            self.do_parse(name, &mut HashSet::new())
        } else {
            self.env.require(name)
        }
    }
}

/// A registry for registering `PropertySource`, which implements `Environment`.
pub struct SourceRegistry {
    sources: Vec<Box<dyn PropertySource>>,
}

impl SourceRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        SourceRegistry { sources: vec![] }
    }

    /// Add default command line arguments parser.
    #[cfg(feature = "enable_args")]
    pub fn with_args(mut self, mode: args::SysArgsMode) -> Self {
        self.register_source(Box::new(args::SysArgs::new(mode).0));
        self
    }

    /// Add system environment.
    pub fn with_sys_env(mut self) -> Self {
        self.register_source(Box::new(env::SysEnv));
        self
    }

    /// Add toml file.
    #[cfg(feature = "enable_toml")]
    pub fn with_toml(mut self) -> Self {
        let dir: Option<String> = self.get("APP_CONF_DIR");
        let name = self.get_or("APP_CONF_NAME", "app".to_owned());
        #[cfg(feature = "enable_log")]
        {
            if let Some(d) = &dir {
                debug!("Set APP_CONF_DIR as {}.", &d);
            }
            debug!("Set APP_CONF_NAME as {}.", name);
        }
        self.register_sources(Toml::new(dir, name).build());
        self
    }

    /// Register source.
    pub fn register_source(&mut self, source: Box<dyn PropertySource>) {
        if !source.is_empty() {
            #[cfg(feature = "enable_log")]
            debug!("Load property source {}.", source.name());
            self.sources.push(source);
        }
    }

    /// Register multiple sources.
    pub fn register_sources(&mut self, sources: Vec<Option<Box<dyn PropertySource>>>) {
        for source in sources.into_iter() {
            if let Some(s) = source {
                self.register_source(s);
            }
        }
    }
}

impl Default for SourceRegistry {
    fn default() -> Self {
        let mut sr = SourceRegistry::new();
        #[cfg(not(test))]
        #[cfg(feature = "enable_args")]
        {
            sr = sr.with_args(args::SysArgsMode::Auto(auto_read_sys_args_param!()));
        }
        sr = sr.with_sys_env();
        #[cfg(feature = "enable_toml")]
        {
            sr = sr.with_toml();
        }
        sr
    }
}

impl Environment for SourceRegistry {
    fn contains(&self, name: &str) -> bool {
        self.sources.iter().any(|a| a.contains_property(name))
    }
    fn require<T: FromProperty>(&self, name: &str) -> Result<T, PropertyError> {
        for ps in self.sources.iter() {
            if let Some(v) = ps.get_property(name) {
                return T::from_property(v);
            }
        }
        Err(PropertyError::ParseFail(format!(
            "Property {} not found",
            name
        )))
    }
}

#[cfg(test)]
mod tests {

    use crate::environment::*;

    #[test]
    fn check() {
        std::env::set_var("v1", "value");
        std::env::set_var("v2", "{v1}");
        std::env::set_var("v3", "{no_found:default}");
        std::env::set_var("v4", "{no_found:{v2}}");
        std::env::set_var("v5", "{no_found:{no_found_2:hello}}");
        std::env::set_var("v6", "hello-{v1}-{v3}-");
        std::env::set_var("v7", "{v7}");
        let env = PlaceHolderEnvironment::new(true, SourceRegistry::default());
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
    }
}
