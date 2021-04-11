//! Factory using salak.
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

#[cfg(feature = "enable_pool")]
mod pool;
#[cfg(feature = "enable_pool")]
#[cfg_attr(docsrs, doc(cfg(feature = "enable_pool")))]
pub use crate::pool::*;

#[cfg(feature = "enable_postgres")]
mod postgres;
#[cfg(feature = "enable_postgres")]
#[cfg_attr(docsrs, doc(cfg(feature = "enable_postgres")))]
pub use crate::postgres::{PostgresConfig, PostgresConnectionManager, PostgresCustomizer};

#[cfg(feature = "enable_redis")]
mod redis;
#[cfg(feature = "enable_redis")]
#[cfg_attr(docsrs, doc(cfg(feature = "enable_redis")))]
pub use crate::redis::{RedisConfig, RedisConnectionManager};

#[cfg(feature = "enable_redis_cluster")]
mod redis_cluster;
#[cfg(feature = "enable_redis_cluster")]
#[cfg_attr(docsrs, doc(cfg(feature = "enable_redis_cluster")))]
pub use crate::redis_cluster::{RedisClusterConfig, RedisClusterConnectionManager};

#[cfg(feature = "enable_log")]
mod tracing_log;
#[cfg(feature = "enable_log")]
#[cfg_attr(docsrs, doc(cfg(feature = "enable_log")))]
pub use crate::tracing_log::*;

/// Default namespace
pub const DEFAULT_NAMESPACE: &str = "primary";

/// Buildable component from [`Environment`].
pub trait Buildable: Sized + FromEnvironment {
    /// Target product.
    type Product;

    /// Customize when building.
    type Customizer: Default;

    /// Configuration prefix.
    fn prefix() -> &'static str;

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

    /// List All Keys
    fn list_keys(namespace: &str) -> Vec<(String, bool, Option<String>)> {
        let v = Self::load_keys();
        let prefix = if namespace.is_empty() || namespace == DEFAULT_NAMESPACE {
            Self::prefix().to_string()
        } else {
            format!("{}.{}", Self::prefix(), namespace)
        };
        v.iter()
            .map(|(k, o, v)| {
                (
                    format!("{}.{}", prefix, k),
                    *o,
                    match v {
                        Some(p) => String::from_property(p.clone()).ok(),
                        _ => None,
                    },
                )
            })
            .collect()
    }
}

#[allow(dead_code)]
fn print_keys<T: Buildable>() {
    println!("/// |property|required|default|");
    println!("/// |-|-|-|");
    for (k, r, v) in T::list_keys(DEFAULT_NAMESPACE) {
        println!("/// |{}|{}|{}|", k, r, v.unwrap_or("".to_owned()));
    }
}

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
