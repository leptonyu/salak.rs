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

pub use crate::raw::*;
mod raw;

pub use crate::env::*;
mod env;

mod enums;

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
    fn get<T: DefaultSourceFromEnvironment>(&self) -> Result<T, PropertyError> {
        self.require::<T>(T::prefix())
    }
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
}
