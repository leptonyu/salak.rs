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
//!   - Support following random property key.
//!     - `random.u8`
//!     - `random.u16`
//!     - `random.u32`
//!     - `random.i8`
//!     - `random.i16`
//!     - `random.i32`
//!     - `random.i64`
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
//! 1. `enable_pico`, enable default command line arguments parsing by `pico-args`.
//! 2. `enable_clap`, enable default command line arguments parsing by `clap`.
//! 3. `enable_yaml`, enable yaml support.
//! 4. `enable_rand`, enable random value support.
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
//! let env = Salak::new()
//!    .with_default_args(auto_read_sys_args_param!()) // This line need enable feature `enable_pico`.
//!     .add_default::<DatabaseConfig>()
//!     .add_default_source(inline_toml!("app.toml"))
//!     .set_property("database.url", "localhost:5432")
//!     .set_property("database.description", "\\$\\{Hello\\}")
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

// use std::collections::HashSet;
// use std::hash::BuildHasher;
// use std::hash::Hash;

// #[cfg(test)]
// #[macro_use(quickcheck)]
// extern crate quickcheck_macros;

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

pub use crate::raw::*;
mod raw;

pub use crate::env::*;
mod env;

pub use crate::err::PropertyError;
pub use crate::utils::SalakStringUtil;

// mod env;
// pub(crate) use crate::env::factory::FactoryRegistry;
// pub use crate::env::{
//     // factory::{FacRef, Factory, FactoryContext, FactoryScope, FromFactory},
//     placeholder::PlaceholderResolver,
//     registry::SourceRegistry,
//     salak::{Salak, SalakBuilder},
// };

pub use crate::source::system_environment;
pub use crate::source::PropertyRegistry;

mod source;
#[cfg(feature = "toml")]
#[cfg_attr(docsrs, doc(cfg(feature = "toml")))]
mod source_toml;
#[cfg(feature = "toml")]
#[cfg_attr(docsrs, doc(cfg(feature = "toml")))]
pub use source_toml::Toml;
#[cfg(feature = "yaml")]
#[cfg_attr(docsrs, doc(cfg(feature = "yaml")))]
mod source_rand;
#[cfg(feature = "rand")]
#[cfg_attr(docsrs, doc(cfg(feature = "rand")))]
mod source_yaml;

#[allow(unused)]
pub(crate) const NOT_POSSIBLE: &str = "Not possible";

/// Raw property.
#[derive(Clone, Debug)]
pub enum Property<'a> {
    /// Str slice
    S(&'a str),
    /// Owned String
    O(String),
    /// Number
    I(i64),
    /// Float
    F(f64),
    /// Bool
    B(bool),
}

/// An abstract source loader from various sources,
/// such as command line arguments, system environment, files, etc.
pub trait PropertySource {
    /// [`PropertySource`] name.
    fn name(&self) -> &str;

    /// Get property by name.
    fn get_property(&self, key: &str) -> Option<Property<'_>>;

    /// Check whether property exists.
    fn contains_key(&self, key: &str) -> bool {
        self.get_property(key).is_some()
    }
    /// Check whether the [`PropertySource`] is empty.
    /// Empty source will not be ignored when register to registry.
    fn is_empty(&self) -> bool;
}

/// An environment for getting properties with mutiple [`PropertySource`]s, placeholder resolve and other features.
pub trait Environment {
    /// Get property with specific type.
    fn require<T: FromEnvironment>(&self, key: &str) -> Result<T, PropertyError> {
        self.require_def(key, None)
    }

    /// Get property with specific type, if property not exists, then return default value.

    fn require_def<T: FromEnvironment>(
        &self,
        key: &str,
        def: Option<Property<'_>>,
    ) -> Result<T, PropertyError>;
}

/// Convert from [`Environment`].
pub trait FromEnvironment: Sized {
    /// Generate object from [`Environment`].
    /// * `key` - Property prefix.
    /// * `property` - Property value with key is `key`.
    /// * `env` - Instance of [`Environment`]
    fn from_env(
        key: &str,
        val: Option<Property<'_>>,
        env: &impl Environment,
    ) -> Result<Self, PropertyError>;
}

fn normalize_key(mut key: &str) -> &str {
    while !key.is_empty() && &key[0..1] == "." {
        key = &key[1..];
    }
    key
}
