//! A resource initialization factory using `salak`.
//! `salak` is a zero-boilerplate configuration parser, it can
//! parsing struct from a unified [`Environment`]. After
//! we got the config struct, we can continue to initialize
//! resource from it. That means we have a unified way to
//! package the initialization process of resources, by specifying
//! configuration properties, and provide a customizer to customize
//! resource by coding.
//!
//! This is how [`Buildable`] works. It provides an interface to
//! initialize target [`Buildable::Resource`] from config struct,
//! which itself can be built by `salak`. Any resource that
//! implements [`Buildable`] can be built by [`Factory`]. And also
//! [`Salak`] is a factory instance.
//!
//!
//! ### Provide Resources
//! 1. redis
//! ```no_run
//! use salak::*;
//! use salak_factory::*;
//! use salak_factory::redis_default::*;
//! let env = Salak::new().unwrap();
//! let redis_pool = env.build::<RedisConfig>().unwrap();
//! ```
//! 2. redis_cluster
//! ```no_run
//! use salak::*;
//! use salak_factory::*;
//! use salak_factory::redis_cluster::*;
//! let env = Salak::new().unwrap();
//! let redis_cluster_pool = env.build::<RedisClusterConfig>().unwrap();
//! ```
//! 3. postgres
//! ```no_run
//! use salak::*;
//! use salak_factory::*;
//! use salak_factory::postgresql::*;
//! let env = Salak::new().unwrap();
//! let pg_pool = env.build::<PostgresConfig>().unwrap();
//! ```

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

use salak::*;

#[cfg(feature = "pool")]
#[cfg_attr(docsrs, doc(cfg(feature = "pool")))]
pub mod pool;

#[cfg(feature = "postgresql")]
#[cfg_attr(docsrs, doc(cfg(feature = "postgresql")))]
pub mod postgresql;

#[cfg(feature = "redis_default")]
#[cfg_attr(docsrs, doc(cfg(feature = "redis_default")))]
pub mod redis_default;

#[cfg(feature = "redis_cluster")]
#[cfg_attr(docsrs, doc(cfg(feature = "redis_cluster")))]
pub mod redis_cluster;

/// Default namespace
pub const DEFAULT_NAMESPACE: &str = "primary";

/// Buildable component from [`Environment`].
///
/// This trait defines standards configuration properties, and
/// the initialization process of target [`Buildable::Resource`].
/// Also it has [`Buildable::Customizer`] to provide coding config,
/// such as set the error handler.
pub trait Buildable: Sized + PrefixedFromEnvironment {
    /// Target resource.
    type Resource;

    /// Customize the resource by coding.
    type Customizer: Default;

    /// Build presource by customizer.
    fn build_with_customizer(
        self,
        customize: Self::Customizer,
    ) -> Result<Self::Resource, PropertyError>;
}

/// Factory defines how to build resource from [`Environment`].
pub trait Factory: Environment {
    /// Build by namespace
    fn build<T: Buildable>(&self) -> Result<T::Resource, PropertyError> {
        self.build_by_namespace::<T>(DEFAULT_NAMESPACE)
    }

    /// Build by namespace
    fn build_by_namespace<T: Buildable>(
        &self,
        namespace: &str,
    ) -> Result<T::Resource, PropertyError> {
        self.build_by_namespace_and_customizer::<T>(namespace, T::Customizer::default())
    }

    /// Build by namespace
    fn build_by_customizer<T: Buildable>(
        &self,
        customizer: T::Customizer,
    ) -> Result<T::Resource, PropertyError> {
        self.build_by_namespace_and_customizer::<T>(DEFAULT_NAMESPACE, customizer)
    }

    /// Build the resource by namespace and customizer.
    /// By using different namespace, we can easily extend the
    /// instances of same resource.
    ///
    /// Default namespace is [`DEFAULT_NAMESPACE`]. If the prefix
    /// of resource property is 'salak', and property is 'key',
    /// then the combined key for default namespace is 'salak.key'.
    /// And if you using a customized namespace, eg 'secondary',
    /// the key is 'salak.secondary.key'.
    fn build_by_namespace_and_customizer<T: Buildable>(
        &self,
        namespace: &str,
        customizer: T::Customizer,
    ) -> Result<T::Resource, PropertyError> {
        let config = if namespace.is_empty() || namespace == DEFAULT_NAMESPACE {
            self.require(T::prefix())
        } else {
            self.require(&format!("{}.{}", T::prefix(), namespace))
        };
        T::build_with_customizer(config?, customizer)
    }
}

impl<T: Environment> Factory for T {}

/// Wrap enum for implement [`EnumProperty`].
#[derive(Debug)]
pub struct WrapEnum<T>(pub(crate) T);
