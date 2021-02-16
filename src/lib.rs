//! A configuration loader inspired by spring-boot.
//!
//! ## About
//! `salak` is a rust version for multilayered configuration loader inspired by
//! [spring-boot](https://docs.spring.io/spring-boot/docs/current/reference/html/spring-boot-features.html#boot-features-external-config).
//! `salak` also has a [haskell version](https://hackage.haskell.org/package/salak).
//!
//! `salak` defines default `PropertySource`s:
//! 1. Arguments using `clap` to parsing `-P, --propery KEY=VALUE`.
//! 2. System Environment
//! 3. app.toml(*) in current dir and $HOME dir. Or if you specify `APP_CONF_DIR` dir, then only load toml in this dir.
//!
//! \* `APP_CONF_NAME` can be specified to rename `app`.
//!
//! ### Placeholder parsing
//! Unlike spring-boot, `salak` use format `{key:default}` to parse value or use default.
//!
//!
//! ### Toml key conversion
//! `salak` use the same key conversion as toml.
//!
//! ```toml
//! [a.b.c]
//! hello = "world"
//! ```
//! means
//! ```toml
//! a.b.c.hello = world
//! ```
//!
//! ## Quick Example
//!
//! ```ignore
//! use salak::*;
//! let env = SalakBuilder::new()
//!    .with_args_param(sys_args_param!())
//!    .build();
//!
//! match env.require::<String>("hello") {
//!     Ok(val) => println!("{}", val),
//!     Err(e) => println!("{}", e),
//! }
//! ```
//!
use crate::environment::*;
use crate::property::*;
#[cfg(feature = "enable_log")]
use log::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::{Display, Error, Formatter};

#[cfg(test)]
#[macro_use(quickcheck)]
extern crate quickcheck_macros;

/// Enable register args in environment.
#[cfg(feature = "enable_args")]
#[macro_use]
pub mod args;
pub mod env;
pub mod environment;
pub mod property;
/// Enable register toml in environment.
#[cfg(feature = "enable_toml")]
pub mod toml;

/// Unified property structure.
#[derive(Clone)]
pub enum Property {
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
}

/// Property Error
#[derive(Debug, PartialEq, Eq)]
pub enum PropertyError {
    ParseFail(String),
    RecursiveParse(String),
}

impl Display for PropertyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match self {
            PropertyError::ParseFail(e) => write!(f, "{}", e),
            PropertyError::RecursiveParse(n) => write!(f, "Recursive parsing property {}.", &n),
        }
    }
}

/// PropertySource is an abstract source loader from various sources,
/// such as arguments, system environment, files, etc.
pub trait PropertySource: Sync + Send {
    /// Name
    fn name(&self) -> String;
    /// Get property with name.
    fn get_property(&self, name: &str) -> Option<Property>;
    /// Check if property with name exists.
    fn contains_property(&self, name: &str) -> bool {
        self.get_property(name).is_some()
    }
    /// Check if the source is empty.
    fn is_empty(&self) -> bool;
}

/// A simple implementation of `PropertySource`.
pub struct MapPropertySource {
    name: String,
    map: HashMap<String, Property>,
}

impl MapPropertySource {
    /// Create a new `MapPropertySource`.
    pub fn new(name: String, map: HashMap<String, Property>) -> Self {
        MapPropertySource { name, map }
    }
}

impl PropertySource for MapPropertySource {
    fn name(&self) -> String {
        self.name.to_owned()
    }

    fn contains_property(&self, name: &str) -> bool {
        self.map.contains_key(name)
    }
    fn get_property(&self, name: &str) -> Option<Property> {
        self.map.get(name).map(|p| p.clone())
    }
    fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

/// `Environment` is an environment for getting properties in multiple `PropertySource`.
pub trait Environment: Sync + Send {
    /// Check if the environment has property.
    fn contains(&self, name: &str) -> bool {
        self.require::<Property>(name).is_ok()
    }
    /// Get required value, or return error.
    fn require<T: FromProperty>(&self, name: &str) -> Result<T, PropertyError>;
    /// Get optional value.
    fn get<T: FromProperty>(&self, name: &str) -> Option<T> {
        self.require(name).ok()
    }
    /// Get value or using default.
    fn get_or<T: FromProperty>(&self, name: &str, default: T) -> T {
        self.get(name).unwrap_or(default)
    }
}

/// Builder for build `Salak`.
pub struct SalakBuilder {
    #[cfg(feature = "enable_args")]
    args_param: Option<args::SysArgsParam>,
    #[cfg(feature = "enable_args")]
    custom_args: Vec<(String, Property)>,
    enable_placeholder: bool,
    enable_default_registry: bool,
}

impl SalakBuilder {
    /// Create default builder.
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "enable_args")]
            args_param: None,
            #[cfg(feature = "enable_args")]
            custom_args: vec![],
            enable_placeholder: true,
            enable_default_registry: true,
        }
    }

    /// Add default command line parser.
    #[cfg(feature = "enable_args")]
    pub fn with_args_param(mut self, param: args::SysArgsParam) -> Self {
        self.args_param = Some(param);
        self
    }

    #[cfg(feature = "enable_args")]
    pub fn with_custom_args(mut self, args: Vec<(String, Property)>) -> Self {
        self.custom_args = args;
        self
    }

    /// Disable placeholder parsing.
    pub fn disable_placeholder(mut self) -> Self {
        self.enable_placeholder = false;
        self
    }

    /// Disable register default property sources.
    pub fn disable_default_registry(mut self) -> Self {
        self.enable_default_registry = false;
        self
    }

    /// Build a `Salak` environment.
    pub fn build(self) -> Salak {
        let sr = if self.enable_default_registry {
            let mut sr = SourceRegistry::new();
            #[cfg(not(test))]
            #[cfg(feature = "enable_args")]
            {
                if let Some(p) = self.args_param {
                    sr = sr.with_args(p);
                }
                sr.register_source(Box::new(args::SysArgs::new(self.custom_args).0));
            }
            sr = sr.with_sys_env();
            #[cfg(feature = "enable_toml")]
            {
                sr = sr.with_toml();
            }
            sr
        } else {
            SourceRegistry::new()
        };
        Salak(PlaceHolderEnvironment::new(self.enable_placeholder, sr))
    }
}

/// Salak implementation for `Environment`.
pub struct Salak(PlaceHolderEnvironment<SourceRegistry>);

impl Salak {
    /// Register property source at last.
    pub fn register_source(&mut self, ps: Box<dyn PropertySource>) {
        self.0.env.register_source(ps);
    }
    /// Register property sources at last.
    pub fn register_sources(&mut self, sources: Vec<Option<Box<dyn PropertySource>>>) {
        self.0.env.register_sources(sources);
    }
}

impl Default for Salak {
    fn default() -> Self {
        SalakBuilder::new().build()
    }
}

impl Environment for Salak {
    fn contains(&self, name: &str) -> bool {
        self.0.contains(name)
    }
    fn require<T>(&self, name: &str) -> Result<T, PropertyError>
    where
        T: FromProperty,
    {
        self.0.require(name)
    }
}
