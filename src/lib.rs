//! Salak is a multi layered configuration loader and zero-boilerplate configuration parser, with many predefined sources.
//!
//! 1. [About](#about)
//! 2. [Quick Start](#quick-start)
//! 3. [Features](#features)
//!    * [Predefined Sources](#predefined-sources)
//!    * [Key Convention](#key-convention)
//!    * [Value Placeholder Parsing](#value-placeholder-parsing)
//!    * [Attributes For Derive](#attributes-for-derive)
//!
//! ## About
//! `salak` is a multi layered configuration loader with many predefined sources. Also it
//! is a zero-boilerplate configuration parser which provides an auto-derive procedure macro
//! to derive [`FromEnvironment`] so that we can parse configuration structs without any additional codes.
//!
//! ## Quick Start
//! A simple example of `salak`:
//!
//! ```
//! use salak::*;
//!
//! #[derive(Debug, FromEnvironment)]
//! #[salak(prefix = "config")]
//! struct Config {
//!     #[salak(default = false)]
//!     verbose: bool,
//!     optional: Option<String>,
//!     #[salak(name = "val")]
//!     value: i64,
//! }
//! let env = Salak::builder()
//!     .set("config.val", "2021")
//!     .unwrap_build();
//! let config = env.get::<Config>().unwrap();
//! assert_eq!(2021, config.value);
//! assert_eq!(None, config.optional);
//! assert_eq!(false, config.verbose);
//! ```
//!
//! ## Features
//!
//! #### Predefined Sources
//! Predefined sources has the following order, [`PropertyRegistry`] will find by sequence of these orders,
//! if the property with specified key is found at the current source, than return immediately. Otherwise,
//! it will search the next source.
//!
//! 1. Random source provides a group of keys can return random values.
//!    * `random.u8`
//!    * `random.u16`
//!    * `random.u32`
//!    * `random.u64`
//!    * `random.u128`
//!    * `random.usize`
//!    * `random.i8`
//!    * `random.i16`
//!    * `random.i32`
//!    * `random.i64`
//!    * `random.i128`
//!    * `random.isize`
//! 2. Custom arguments source. [`SalakBuilder::set()`] can set a single kv,
//! and [`SalakBuilder::set_args()`] can set a group of kvs.
//! 3. System environment source. Implemented by [`system_environment`].
//! 4. Profile specified file source, eg. `app-dev.toml`
//! 5. Not profile file source, eg. `app.toml`
//! 6. Custom sources, which can register by [`Salak::register()`].
//!
//! #### Key Convention
//! Key is used for search configuration from [`Environment`], normally it is represented by string.
//! Key is a group of SubKey separated by dot(`.`), and SubKey is a name or a name followed by index.
//! 1. SubKey Format (`[a-z][_a-z0-9]+(\[[0-9]+\])*`)
//!    * `a`
//!    * `a0`
//!    * `a_b`
//!    * `a[0]`
//!    * `a[0][0]`
//! 2. Key Format (`SubKey(\.SubKey)*`)
//!    * `a`
//!    * `a.b`
//!    * `a.val[0]`
//!    * `a_b[0]`
//!
//! #### Value Placeholder Parsing
//! 1. Placeholder Format
//!    * `${key}` => Get value of `key`.
//!    * `${key:default}` => Get value of `key`, if not exists return `default`.
//! 2. Escape Format
//!    * `\$\{key\}` => Return `${key}`.
//!    * `$`, `\`, `{`, `}` must use escape format.
//!
//! #### Attributes For Derive
//! `salak` supports some attributes for automatically derive [`FromEnvironment`].
//! All attributes have format `#[salak(..)]`, eg. `#[salak(default = "default value")]`.
//! 1. Struct Header Attribute.
//!    * `#[salak(prefix = "salak.application")]`, has this attr will auto implement [`PrefixedFromEnvironment`].
//! 2. Struct Field Attribute.
//!    * `#[salak(default = "value")]`, this attr can specify default value.
//!    * `#[salak(name = "key")]`, this attr can specify property key, default convension is use field name.
//!    * `#[salak(desc = "Field Description")]`, this attr can be describe this property.
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

#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
mod derive;

#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
pub use crate::derive::{AutoDeriveFromEnvironment, KeyDesc, PrefixedFromEnvironment};
/// Auto derive [`FromEnvironment`] for struct.
#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
pub use salak_derive::FromEnvironment;

#[cfg(feature = "args")]
#[cfg_attr(docsrs, doc(cfg(feature = "args")))]
mod args;
#[cfg(feature = "args")]
#[cfg_attr(docsrs, doc(cfg(feature = "args")))]
pub use crate::args::*;

mod err;
pub use crate::raw::*;
mod raw;
pub use crate::env::*;
mod enums;
mod env;

pub use crate::enums::EnumProperty;
pub use crate::err::{PropertyError, SalakParseError};
pub use crate::source::{system_environment, HashMapSource, PropertyRegistry};

mod source;
#[cfg(feature = "toml")]
#[cfg_attr(docsrs, doc(cfg(feature = "toml")))]
mod source_toml;
#[cfg(feature = "toml")]
#[cfg_attr(docsrs, doc(cfg(feature = "toml")))]
pub use source_toml::Toml;
#[cfg(feature = "rand")]
#[cfg_attr(docsrs, doc(cfg(feature = "rand")))]
mod source_rand;
#[cfg(feature = "yaml")]
#[cfg_attr(docsrs, doc(cfg(feature = "yaml")))]
mod source_yaml;

#[cfg(test)]
#[macro_use(quickcheck)]
extern crate quickcheck_macros;

/// An abstract source loader from various sources,
/// such as command line arguments, system environment, files, etc.
pub trait PropertySource {
    /// [`PropertySource`] name.
    fn name(&self) -> &str;

    /// Get property by name.
    fn get_property(&self, key: &Key<'_>) -> Option<Property<'_>>;

    /// Return next sub keys with prefix, sub keys are seperated by dot(.) in a key.
    fn sub_keys<'a>(&'a self, key: &Key<'_>, sub_keys: &mut SubKeys<'a>);

    /// Check whether the [`PropertySource`] is empty.
    /// Empty source will not be ignored when register to registry.
    fn is_empty(&self) -> bool;
}

/// An environment for getting properties with mutiple [`PropertySource`]s, placeholder resolve and other features.
pub trait Environment {
    /// Get config with specific type.
    fn require<T: FromEnvironment>(&self, key: &str) -> Result<T, PropertyError> {
        self.require_def(&mut Key::new(), SubKey::S(key), None)
    }

    /// Get config with specific type, if config not exists, then return default value.
    /// * `key` - Property key.
    /// * `sub_key` - Property sub key.
    /// * `def` - Default property.
    fn require_def<'a, T: FromEnvironment, K: Into<SubKey<'a>>>(
        &'a self,
        key: &mut Key<'a>,
        sub_key: K,
        def: Option<Property<'_>>,
    ) -> Result<T, PropertyError>;

    #[doc(hidden)]
    fn sub_keys<'a>(&'a self, prefix: &Key<'_>, sub_keys: &mut SubKeys<'a>);

    #[cfg(feature = "derive")]
    #[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
    /// Get config with predefined prefix.
    fn get<T: PrefixedFromEnvironment>(&self) -> Result<T, PropertyError> {
        self.require::<T>(T::prefix())
    }

    /// Get key description.
    #[cfg(feature = "derive")]
    #[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
    fn get_desc<T: PrefixedFromEnvironment>(&self) -> Vec<KeyDesc> {
        let mut keys = vec![];
        self.key_desc::<T, &str>(&mut Key::new(), T::prefix(), None, None, None, &mut keys);
        keys
    }

    #[cfg(feature = "derive")]
    #[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
    #[doc(hidden)]
    fn key_desc<'a, T: FromEnvironment, K: Into<SubKey<'a>>>(
        &'a self,
        key: &mut Key<'a>,
        sub_key: K,
        required: Option<bool>,
        def: Option<&'a str>,
        desc: Option<String>,
        keys: &mut Vec<KeyDesc>,
    );
}

/// Convert from [`Environment`].
pub trait FromEnvironment: Sized {
    /// Generate object from [`Environment`].
    /// * `key` - Property key.
    /// * `property` - Property value with key is `key`.
    /// * `env` - Instance of [`Environment`]
    fn from_env<'a>(
        key: &mut Key<'a>,
        val: Option<Property<'_>>,
        env: &'a impl Environment,
    ) -> Result<Self, PropertyError>;

    #[cfg(feature = "derive")]
    #[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
    #[doc(hidden)]
    fn key_desc<'a>(
        key: &mut Key<'a>,
        desc: &mut KeyDesc,
        keys: &mut Vec<KeyDesc>,
        env: &'a impl Environment,
    );
}
