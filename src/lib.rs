//! Salak is a zero-bioplate configuration loader with predefined  multiple layered sources.
//!
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
pub use crate::derive::{AutoDeriveFromEnvironment, DefaultSourceFromEnvironment};
/// Auto derive [`FromEnvironment`] for struct.
#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
pub use salak_derive::FromEnvironment;

mod err;
mod utils;

pub use crate::raw::*;
mod raw;

pub use crate::env::*;
mod env;

pub use crate::err::PropertyError;
pub use crate::source::system_environment;
pub use crate::source::PropertyRegistry;
pub use crate::utils::SalakStringUtil;

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

    /// Return next sub keys with prefix, sub keys are seperated by dot(.) in a key.
    fn sub_keys(&self, prefix: &str) -> Vec<&str>;

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
