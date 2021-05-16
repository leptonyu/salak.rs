//! Packages that can be initialized by `salak`.
//!
//! ### Provide packages
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

// #[cfg(feature = "enable_log")]
// #[cfg_attr(docsrs, doc(cfg(feature = "enable_log")))]
// mod toy_log;
// #[cfg(feature = "enable_log")]
// #[cfg_attr(docsrs, doc(cfg(feature = "enable_log")))]
// pub use crate::toy_log::*;

/// Default namespace
pub const DEFAULT_NAMESPACE: &str = "primary";

/// Buildable component from [`Environment`].
pub trait Buildable: Sized + PrefixedFromEnvironment {
    /// Target product.
    type Product;

    /// Customize when building.
    type Customizer: Default;

    /// Build product.
    fn build(
        namespace: &str,
        env: &impl Environment,
        customize: Self::Customizer,
    ) -> Result<Self::Product, PropertyError> {
        let config = if namespace.is_empty() || namespace == DEFAULT_NAMESPACE {
            env.require(Self::prefix())
        } else {
            env.require(&format!("{}.{}", Self::prefix(), namespace))
        };
        Self::build_with_key(config?, env, customize)
    }

    /// Build with specified prefix.
    fn build_with_key(
        self,
        env: &impl Environment,
        customize: Self::Customizer,
    ) -> Result<Self::Product, PropertyError>;

    // /// List All Keys
    // fn list_keys(namespace: &str) -> Vec<(String, bool, Option<String>)> {
    //     let v = Self::load_keys();
    //     let prefix = if namespace.is_empty() || namespace == DEFAULT_NAMESPACE {
    //         Self::prefix().to_string()
    //     } else {
    //         format!("{}.{}", Self::prefix(), namespace)
    //     };
    //     v.iter()
    //         .map(|(k, o, v)| {
    //             (
    //                 format!("{}.{}", prefix, k),
    //                 *o,
    //                 match v {
    //                     Some(p) => std::convert::TryInto::<String>::try_into(p.clone()).ok(),
    //                     _ => None,
    //                 },
    //             )
    //         })
    //         .collect()
    // }
}

// #[allow(dead_code)]
// fn print_keys<T: Buildable>() {
//     println!("/// |property|required|default|");
//     println!("/// |-|-|-|");
//     for (k, r, v) in T::list_keys(DEFAULT_NAMESPACE) {
//         println!("/// |{}|{}|{}|", k, r, v.unwrap_or("".to_owned()));
//     }
// }

/// Factory for build buildable
pub trait Factory: Environment {
    /// Build by namespace
    fn build<T: Buildable>(&self) -> Result<T::Product, PropertyError> {
        self.build_by_namespace::<T>(DEFAULT_NAMESPACE)
    }

    /// Build by namespace
    fn build_by_namespace<T: Buildable>(
        &self,
        namespace: &str,
    ) -> Result<T::Product, PropertyError> {
        self.build_by_namespace_and_customizer::<T>(namespace, T::Customizer::default())
    }

    /// Build by namespace
    fn build_by_customizer<T: Buildable>(
        &self,
        customizer: T::Customizer,
    ) -> Result<T::Product, PropertyError> {
        self.build_by_namespace_and_customizer::<T>(DEFAULT_NAMESPACE, customizer)
    }

    /// Build by namespace and customizer
    fn build_by_namespace_and_customizer<T: Buildable>(
        &self,
        namespace: &str,
        customizer: T::Customizer,
    ) -> Result<T::Product, PropertyError>;
}

impl Factory for Salak {
    fn build_by_namespace_and_customizer<T: Buildable>(
        &self,
        namespace: &str,
        customizer: T::Customizer,
    ) -> Result<T::Product, PropertyError> {
        T::build(namespace, self, customizer)
    }
}

/// Wrap enum for implement enum.
#[doc(hidden)]
#[derive(Debug)]
pub struct WrapEnum<T>(pub(crate) T);
