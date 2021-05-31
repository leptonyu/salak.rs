//! Salak is a multi layered configuration loader and zero-boilerplate configuration parser, with many predefined sources.
//!
//! 1. [About](#about)
//! 2. [Quick Start](#quick-start)
//! 3. [Features](#features)
//!    * [Predefined Sources](#predefined-sources)
//!    * [Key Convention](#key-convention)
//!    * [Value Placeholder Parsing](#value-placeholder-parsing)
//!    * [Attributes For Derive](#attributes-for-derive)
//!    * [Reload Configuration](#reload-configuration)
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
//!     .build()
//!     .unwrap();
//! let config = env.get::<Config>().unwrap();
//! assert_eq!(2021, config.value);
//! assert_eq!(None, config.optional);
//! assert_eq!(false, config.verbose);
//! ```
//!
//! ## Features
//!
//! #### Predefined Sources
//! Predefined sources has the following order, [`Salak`] will find by sequence of these orders,
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
//! 3. System environment source. Implemented by [`source::system_environment`].
//! 4. Profile specified file source, eg. `app-dev.toml`, supports reloading.
//! 5. No profile file source, eg. `app.toml`, supports reloading.
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
//! #### Reload Configuration
//! `salak` supports reload configurations. Since in rust mutable
//! and alias can't be used together, here we introduce a wrapper
//! [`wrapper::IORef`] for updating values when reloading.
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

use std::sync::Mutex;

#[cfg(feature = "derive")]
use crate::derive::KeyDesc;
#[cfg(feature = "derive")]
mod derive;
#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
pub use crate::derive::{
    AutoDeriveFromEnvironment, DescFromEnvironment, PrefixedFromEnvironment, SalakDescContext,
};
use raw_ioref::IORefT;
/// Auto derive [`FromEnvironment`] for struct.
#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
pub use salak_derive::FromEnvironment;
use source_raw::PropertyRegistryInternal;

#[cfg(feature = "args")]
#[cfg_attr(docsrs, doc(cfg(feature = "args")))]
mod args;
#[cfg(feature = "args")]
#[cfg_attr(docsrs, doc(cfg(feature = "args")))]
pub use crate::args::AppInfo;

mod err;
mod raw;
use crate::raw::SubKey;
pub use crate::raw::{IsProperty, Property};
mod raw_ioref;
mod raw_vec;
use crate::env::PREFIX;
pub use crate::env::{Salak, SalakBuilder};
mod enums;
mod env;

pub use crate::enums::EnumProperty;
pub use crate::err::PropertyError;

mod source_map;
#[cfg(feature = "rand")]
#[cfg_attr(docsrs, doc(cfg(feature = "rand")))]
mod source_rand;
mod source_raw;
#[cfg(feature = "toml")]
#[cfg_attr(docsrs, doc(cfg(feature = "toml")))]
mod source_toml;
#[cfg(feature = "yaml")]
#[cfg_attr(docsrs, doc(cfg(feature = "yaml")))]
mod source_yaml;

use crate::source::Key;
use crate::source::SubKeys;

#[cfg(test)]
#[macro_use(quickcheck)]
extern crate quickcheck_macros;

/// Salak wrapper for configuration parsing.
///
/// Wrapper can determine extra behavior for parsing.
/// Such as check empty of vec or update when reloading.
pub mod wrapper {
    pub use crate::raw_ioref::IORef;
    pub use crate::raw_vec::NonEmptyVec;
}

/// Salak sources.
///
/// This mod exports all pub sources.
pub mod source {

    #[cfg(feature = "args")]
    #[cfg_attr(docsrs, doc(cfg(feature = "args")))]
    pub(crate) use crate::args::from_args;
    pub use crate::raw::Key;
    pub use crate::raw::SubKeys;
    pub use crate::source_map::system_environment;
    pub use crate::source_map::HashMapSource;
}

/// A property source defines how to load properties.
/// `salak` has some predefined sources, user can
/// provide custom source by implementing this trait.
///
/// Sources provided by `salak`.
///
/// * hashmap source
/// * std::env source
/// * toml source
/// * yaml source
pub trait PropertySource: Send + Sync {
    /// [`PropertySource`] name.
    fn name(&self) -> &str;

    /// Get property by key.
    fn get_property(&self, key: &Key<'_>) -> Option<Property<'_>>;

    /// Get all subkeys with given key.
    ///
    /// Subkeys are keys without dot('.').
    /// This method is unstable, and will be simplified by hidding
    /// Key and SubKeys.
    fn get_sub_keys<'a>(&'a self, key: &Key<'_>, sub_keys: &mut SubKeys<'a>);

    /// Check whether the [`PropertySource`] is empty.
    /// Empty source will be ignored when registering to `salak`.
    fn is_empty(&self) -> bool;

    /// Reload source, if nothing changes, then return none.
    #[inline]
    fn reload_source(&self) -> Result<Option<Box<dyn PropertySource>>, PropertyError> {
        Ok(None)
    }
}

/// Environment defines interface for getting values, and reloading
/// configurations.
///
/// The implementor of this trait is [`Salak`].
pub trait Environment {
    /// Get value by key.
    /// * `key` - Configuration key.
    ///
    /// Require means is if the value `T` is not found,
    /// then error will be returned. But if you try to get
    /// `Option<T>`, then not found will return `None`.
    fn require<T: FromEnvironment>(&self, key: &str) -> Result<T, PropertyError>;

    /// Reload configuration. If reloading is completed,
    /// all values wrapped by [`wrapper::IORef`] will be updated.
    ///
    /// Currently, this feature is unstable, the returned bool
    /// value means reloading is completed without error.
    fn reload(&self) -> Result<bool, PropertyError>;

    #[cfg(feature = "derive")]
    #[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
    #[inline]
    /// Get value with predefined key.
    ///
    /// [`PrefixedFromEnvironment`] can be auto derives by
    /// [`salak_derive::FromEnvironment`] macro. It provides
    /// a standard key for getting value `T`.
    fn get<T: PrefixedFromEnvironment>(&self) -> Result<T, PropertyError> {
        self.require::<T>(T::prefix())
    }
}

/// Context for implementing [`FromEnvironment`].
#[allow(missing_debug_implementations)]
pub struct SalakContext<'a> {
    registry: &'a PropertyRegistryInternal<'a>,
    iorefs: &'a Mutex<Vec<Box<dyn IORefT + Send>>>,
    key: &'a mut Key<'a>,
}

/// Parsing value from environment by [`SalakContext`].
pub trait FromEnvironment: Sized {
    /// Generate object from [`SalakContext`].
    /// * `val` - Property value can be parsed from.
    /// * `env` - Context.
    ///
    /// ```no_run
    /// use salak::*;
    /// pub struct Config {
    ///   key: String
    /// }
    /// impl FromEnvironment for Config {
    ///   fn from_env(
    ///       val: Option<Property<'_>>,
    ///       env: &mut SalakContext<'_>,
    ///   ) -> Result<Self, PropertyError> {
    ///     Ok(Self{
    ///       key: env.require_def("key", None)?,
    ///     })
    ///   }
    /// }
    ///
    /// ```
    fn from_env(
        val: Option<Property<'_>>,
        env: &mut SalakContext<'_>,
    ) -> Result<Self, PropertyError>;
}
