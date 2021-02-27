//! A layered configuration loader with zero-boilerplate configuration management.
//!
//! 1. [About](#about)
//! 2. [Features](#features)
//! 3. [Placeholder](#placeholder)
//! 4. [Key Convension](#key-convension)
//! 5. [Cargo Features](#cargo-features)
//!     1. [Default features](#default-features)
//!     2. [Optional features](#optional-features)
//! 6. [Quick Example](#quick-example)
//!
//!
//! ## About
//! `salak` is a rust version of layered configuration loader inspired by
//! [spring-boot](https://docs.spring.io/spring-boot/docs/current/reference/html/spring-boot-features.html#boot-features-external-config).
//! `salak` provide an [`Environment`] structure which load properties from various [`PropertySource`]s.
//! Any structure which impmement [`FromEnvironment`] can get from [`Environment`] by a key.
//! Feature `enable_derive` provide rust attributes for auto derive [`FromEnvironment`].
//!
//! ## Features
//! Below are a few of the features which `salak` supports.
//!
//! * Auto mapping properties into configuration struct.
//!   - `#[salak(default="value")]` set default value.
//!   - `#[salak(name="key")]` rename property key.
//!   - `#[salak(prefix="salak.database")]` set prefix.
//! * ** Supports load properties from various sources **
//!   - Load properties from command line arguments.
//!   - Load properties from system environment.
//!   - Load properties from toml config file.
//!   - Load properties from yaml config file.
//!   - Easy to add a new property source.
//! * Supports profile(develop/production) based configuration.
//! * Supports placeholder resolve.
//! * Supports reload configurations.
//!
//! ## Placeholder
//!
//! * `${key:default}` means get value of `key`, if not exists then return `default`.
//! * `${key}` means get value of `key`, if not exists then return `PropertyError::NotFound(_)`.
//! * `\$\{key\}` means escape to `${key}`.
//!
//! ## Key Convension
//! * `a.b.c` is a normal key separated by dot(`.`).
//! * `a.b[0]`, `a.b[1]`, `a.b[2]`... is a group of keys with arrays.
//!
//! ## Cargo Features
//!
//! ### Default features
//! 1. `enable_log`, enable log record if enabled.
//! 2. `enable_toml`, enable toml support.
//! 3. `enable_derive`, enable auto derive [`FromEnvironment`] for struts.
//!
//! ### Optional features
//! 1. `enable_clap`, enable default command line arguments parsing by `clap`.
//! 2. `enable_yaml`, enable yaml support.
//!
//! ## Quick Example
//!
//! ```
//! use salak::*;
//!
//! #[derive(FromEnvironment, Debug)]
//! pub struct SslConfig {
//!     key: String,
//!     pem: String,
//! }
//!
//! #[derive(FromEnvironment, Debug)]
//! #[salak(prefix = "database")]
//! pub struct DatabaseConfig {
//!   url: String,
//!   #[salak(default = "salak")]
//!   username: String,
//!   password: Option<String>,
//!   description: String,
//!   #[salak(name="ssl")]
//!   ssl_config: Option<SslConfig>,  
//! }
//!
//! std::env::set_var("database.url", "localhost:5432");
//! std::env::set_var("database.description", "\\$\\{Hello\\}");
//! let env = Salak::new()
//!    .with_default_args(auto_read_sys_args_param!()) // This line need enable feature `enable_clap`.
//!    .build();
//!
//! match env.load_config::<DatabaseConfig>() {
//!     Ok(val) => println!("{:?}", val),
//!     Err(e) => println!("{}", e),
//! }
//!
//! // Output: DatabaseConfig {
//! //  url: "localhost:5432",
//! //  username: "salak",
//! //  password: None,
//! //  description: "${Hello}",
//! //  ssl_config: None,
//! // }
//! ```
//!
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(
    anonymous_parameters,
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    nonstandard_style,
    rust_2018_idioms,
    single_use_lifetimes,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    unused_extern_crates,
    unused_qualifications,
    variant_size_differences
)]

#[cfg(feature = "enable_log")]
use log::*;
use std::collections::HashSet;
use std::hash::BuildHasher;
use std::hash::Hash;

#[cfg(test)]
#[macro_use(quickcheck)]
extern crate quickcheck_macros;

#[cfg(feature = "enable_derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "enable_derive")))]
mod derive;

#[cfg(feature = "enable_derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "enable_derive")))]
pub use crate::derive::{AutoDeriveFromEnvironment, DefaultSourceFromEnvironment};
/// Auto derive [`FromEnvironment`] for struct.
#[cfg(feature = "enable_derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "enable_derive")))]
pub use salak_derive::FromEnvironment;

mod err;
mod utils;

pub use crate::err::PropertyError;
pub use crate::utils::SalakStringUtil;

mod env;
pub use crate::env::{
    placeholder::PlaceholderResolver,
    registry::SourceRegistry,
    salak::{Salak, SalakBuilder},
};

mod source;

#[cfg(feature = "enable_toml")]
#[cfg_attr(docsrs, doc(cfg(feature = "enable_toml")))]
pub use crate::source::toml::Toml;
#[cfg(feature = "enable_yaml")]
#[cfg_attr(docsrs, doc(cfg(feature = "enable_yaml")))]
pub use crate::source::yaml::Yaml;
pub use crate::source::{args::*, env::SysEnvPropertySource, map::MapPropertySource};

/// Raw property.
#[derive(Clone, Debug)]
pub enum Property {
    /// String value.
    Str(String),
    /// Integer value.
    Int(i64),
    /// Float value.
    Float(f64),
    /// Bool value.
    Bool(bool),
}

/// Convert to [`Property`].
pub trait IntoProperty: Sized {
    /// Convert to property.
    fn into_property(self) -> Property;
}

/// Convert from [`Property`].
pub trait FromProperty: Sized {
    /// Convert from property.
    fn from_property(_: Property) -> Result<Self, PropertyError>;
}

/// An abstract source loader from various sources,
/// such as command line arguments, system environment, files, etc.
pub trait PropertySource: Sync + Send {
    /// [`PropertySource`] name.
    fn name(&self) -> String;
    /// Get property by name.
    fn get_property(&self, key: &str) -> Option<Property>;
    /// Check whether property exists.
    fn contains_property(&self, key: &str) -> bool {
        self.get_property(key).is_some()
    }
    /// Check whether the [`PropertySource`] is empty.
    /// Empty source will not be ignored when register to registry.
    fn is_empty(&self) -> bool;

    /// Find all next level keys with prefix.
    fn get_keys(&self, prefix: &str) -> Vec<String>;

    /// Reload [`PropertySource`], if this [`PropertySource`] not support reload, then just return `Ok(None)`.
    fn load(&self) -> Result<Option<Box<dyn PropertySource>>, PropertyError>;
}

/// An environment for getting properties with mutiple [`PropertySource`]s, placeholder resolve and other features.
pub trait Environment: Sync + Send + Sized {
    /// Check whether property exists.
    fn contains(&self, key: &str) -> bool {
        self.require::<Property>(key).is_ok()
    }
    /// Get property with specific type.
    fn require<T: FromEnvironment>(&self, key: &str) -> Result<T, PropertyError>;

    /// Get property with specific type, if property not exists, then return default value.
    fn require_or<T: FromEnvironment>(&self, key: &str, default: T) -> Result<T, PropertyError> {
        match self.require::<Option<T>>(key) {
            Ok(Some(a)) => Ok(a),
            Ok(None) => Ok(default),
            Err(e) => Err(e),
        }
    }
    /// Get property with specific type, if error happens then return [`None`].
    fn get<T: FromEnvironment>(&self, key: &str) -> Option<T> {
        self.require(key).ok()
    }
    /// Get property with specific type, if error happens then return default value.
    fn get_or<T: FromEnvironment>(&self, key: &str, default: T) -> T {
        self.get(key).unwrap_or(default)
    }

    /// Resolve placeholder value.
    fn resolve_placeholder(&self, value: String) -> Result<Option<Property>, PropertyError>;

    /// Load properties which has `#[salak(prefix="prefix")]`
    #[cfg(feature = "enable_derive")]
    #[cfg_attr(docsrs, doc(cfg(feature = "enable_derive")))]
    fn load_config<T: DefaultSourceFromEnvironment>(&self) -> Result<T, PropertyError> {
        self.require(T::prefix())
    }

    /// Find all next level keys with prefix.
    fn find_keys(&self, prefix: &str) -> Vec<String>;

    /// Reload [`Environment`].
    fn reload(&mut self) -> Result<(), PropertyError>;
}

/// Convert from [`Environment`].
pub trait FromEnvironment: Sized {
    /// Generate object from [`Environment`].
    /// * `prefix` - Property prefix.
    /// * `property` - Property value with key is `prefix`.
    /// * `env` - Instance of [`Environment`]
    fn from_env(
        prefix: &str,
        property: Option<Property>,
        env: &impl Environment,
    ) -> Result<Self, PropertyError>;

    /// Empty check for some containers, such as [`Vec<T>`] or [`Option<T>`].
    fn check_is_empty(&self) -> bool {
        false
    }

    #[doc(hidden)]
    /// Handle special case such as property not found.
    fn from_err(err: PropertyError) -> Result<Self, PropertyError> {
        Err(err)
    }

    #[doc(hidden)]
    #[cfg(feature = "enable_derive")]
    fn load_default() -> Vec<(String, Property)> {
        vec![]
    }
}

#[cfg(feature = "enable_toml")]
#[cfg(feature = "enable_derive")]
#[cfg(test)]
mod tests {
    use crate::*;
    use std::collections::HashMap;
    #[derive(FromEnvironment, Debug)]
    struct DatabaseConfigObj {
        hello: String,
        world: Option<String>,
    }
    #[derive(FromEnvironment, Debug)]
    struct DatabaseConfigDetail {
        #[salak(default = "str")]
        option_str: String,
        #[salak(default = 1)]
        option_i64: i64,
        option_arr: Vec<i64>,
        option_multi_arr: Vec<Vec<i64>>,
        option_obj: Vec<DatabaseConfigObj>,
    }

    #[derive(FromEnvironment, Debug)]
    #[salak(prefix = "database")]
    struct DatabaseConfig {
        url: String,
        name: String,
        #[salak(default = "${database.name}")]
        username: String,
        password: Option<String>,
        description: String,
        detail: DatabaseConfigDetail,
    }
    #[test]
    fn integration_tests() {
        let env = Salak::new()
            .with_custom_args(vec![
                (
                    "database.detail.option_arr[0]".to_owned(),
                    "10".into_property(),
                ),
                ("database.url".to_owned(), "localhost:5432".into_property()),
                ("database.name".to_owned(), "salak".into_property()),
                (
                    "database.description".to_owned(),
                    "\\$\\{Hello\\}".into_property(),
                ),
            ])
            .build();

        let ret = env.load_config::<DatabaseConfig>();
        assert_eq!(true, ret.is_ok());
        let ret = ret.unwrap();
        assert_eq!("localhost:5432", ret.url);
        assert_eq!("salak", ret.name);
        assert_eq!("salak", ret.username);
        assert_eq!(None, ret.password);
        assert_eq!("${Hello}", ret.description);
        let ret = ret.detail;
        assert_eq!("str", ret.option_str);
        assert_eq!(1, ret.option_i64);
        assert_eq!(5, ret.option_arr.len());
        assert_eq!(10, ret.option_arr[0]);
        assert_eq!(0, ret.option_multi_arr.len());
        assert_eq!(2, ret.option_obj.len());

        let ret = env.require::<HashMap<String, String>>("database");
        assert_eq!(true, ret.is_ok());
        let ret = ret.unwrap();
        assert_eq!(3, ret.len());
    }
}
