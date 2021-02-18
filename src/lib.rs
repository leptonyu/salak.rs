//! A configuration loader, and zero-boilerplate configuration management.
//!
//! ## About
//! `salak` is a rust version for multi-layered configuration loader inspired by
//! [spring-boot](https://docs.spring.io/spring-boot/docs/current/reference/html/spring-boot-features.html#boot-features-external-config).
//! `salak` also has a [haskell version](https://hackage.haskell.org/package/salak).
//!
//! `salak` defines following default [`PropertySource`]s:
//! 1. Command line arguments using `clap` to parsing `-P, --propery KEY=VALUE`.
//! 2. System Environment.
//! 3. app.toml(*) in current dir and $HOME dir. Or if you specify `APP_CONF_DIR` dir, then only load toml in this dir.
//!
//! \* `APP_CONF_NAME` can be specified to replace `app`.
//!
//! ### Placeholder parsing
//! `salak` use format `{key:default}` to reference to other `key`, and if `key` not exists then use value `default`.
//!
//! ### Toml key conversion
//! `salak` use the same key conversion as toml.
//!
//! ## Quick Example
//!
//! ```
//! use salak::*;
//! #[derive(FromEnvironment, Debug)]
//! pub struct DatabaseConfig {
//!     url: String,
//!     #[field(default = "salak")]
//!     username: String,
//!     password: Option<String>,
//! }
//!
//! fn main() {
//!   std::env::set_var("database.url", "localhost:5432");
//!   let env = SalakBuilder::new()
//!      .with_default_args(auto_read_sys_args_param!())
//!      .build();
//!  
//!   match env.require::<DatabaseConfig>("database") {
//!       Ok(val) => println!("{:?}", val),
//!       Err(e) => println!("{}", e),
//!   }
//! }
//! ```
//!

use crate::map::MapPropertySource;
use crate::property::*;
#[cfg(feature = "enable_log")]
use log::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::{Display, Error, Formatter};

#[cfg(test)]
#[macro_use(quickcheck)]
extern crate quickcheck_macros;

#[cfg(feature = "enable_derive")]
/// Auto derive [`FromEnvironment`] for struct.
pub use salak_derive::FromEnvironment;

/// Enable register args in environment.
#[cfg(feature = "enable_args")]
#[macro_use]
pub mod args;
pub mod env;
mod environment;
pub mod map;
pub mod property;
/// Enable register toml in environment.
#[cfg(feature = "enable_toml")]
pub mod toml;

pub use crate::environment::{PlaceholderResolver, Salak, SalakBuilder, SourceRegistry};

/// Unified property structure.
#[derive(Clone, Debug)]
pub enum Property {
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
}

/// Property Error
#[derive(Debug, PartialEq, Eq)]
pub enum PropertyError {
    NotFound(String),
    ParseFail(String),
    RecursiveParse(String),
}

impl Display for PropertyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match self {
            PropertyError::NotFound(n) => write!(f, "Property {} not found.", n),
            PropertyError::ParseFail(e) => write!(f, "{}", e),
            PropertyError::RecursiveParse(n) => write!(f, "Property {} recursive.", &n),
        }
    }
}

/// An abstract source loader from various sources,
/// such as commandline arguments, system environment, files, etc.
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

/// An environment for getting properties in multiple [`PropertySource`]s.
pub trait Environment: Sync + Send + Sized {
    /// Check if the environment has property.
    fn contains(&self, name: &str) -> bool {
        self.require::<Property>(name).is_ok()
    }
    /// Get required value, or return error.
    fn require<T: FromEnvironment>(&self, name: &str) -> Result<T, PropertyError> {
        self.require_with_defaults(
            name,
            &mut MapPropertySource::new("default".to_owned(), HashMap::new()),
        )
    }

    fn require_with_defaults<T: FromEnvironment>(
        &self,
        name: &str,
        defs: &mut MapPropertySource,
    ) -> Result<T, PropertyError>;

    /// Get required value, if not exists then return default value, otherwise return error.
    fn require_or<T: FromEnvironment>(&self, name: &str, default: T) -> Result<T, PropertyError> {
        match self.require::<Option<T>>(name) {
            Ok(Some(a)) => Ok(a),
            Ok(None) => Ok(default),
            Err(e) => Err(e),
        }
    }

    /// Get optional value, this function will ignore property parse error.
    fn get<T: FromEnvironment>(&self, name: &str) -> Option<T> {
        self.require(name).ok()
    }
    /// Get value or using default, this function will ignore property parse error.
    fn get_or<T: FromEnvironment>(&self, name: &str, default: T) -> T {
        self.get(name).unwrap_or(default)
    }
}

/// Generate object from [`Environment`].
pub trait FromEnvironment: Sized {
    /// Generate object from env.
    fn from_env(
        prefix: &str,
        p: Option<Property>,
        env: &impl Environment,
        defs: &mut MapPropertySource,
    ) -> Result<Self, PropertyError>;

    /// Handle special case such as property not found.
    fn from_err(err: PropertyError) -> Result<Self, PropertyError> {
        Err(err)
    }
}

impl<P: FromProperty> FromEnvironment for P {
    fn from_env(
        n: &str,
        property: Option<Property>,
        _: &impl Environment,
        _: &mut MapPropertySource,
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
        defs: &mut MapPropertySource,
    ) -> Result<Self, PropertyError> {
        match P::from_env(n, property, env, defs) {
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
}
