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
pub use crate::pool::*;

#[cfg(feature = "enable_postgres")]
mod postgres;
#[cfg(feature = "enable_postgres")]
pub use crate::postgres::{PostgresConfig, PostgresConnectionManager};

#[cfg(feature = "enable_redis")]
mod redis;
#[cfg(feature = "enable_redis")]
pub use crate::redis::{RedisConfig, RedisConnectionManager};

/// Buildable component from [`Environment`].
pub trait Buildable: Sized + FromEnvironment {
    /// Target product.
    type Product;

    /// Configuration prefix.
    fn prefix() -> &'static str;

    /// Build product.
    fn build(namespace: &str, env: &impl Environment) -> Result<Self::Product, PropertyError> {
        let config = if namespace == "primary" {
            env.require(Self::prefix())
        } else {
            env.require(&format!("{}.{}", Self::prefix(), namespace))
        };
        Self::build_with_key(config?, env)
    }

    /// Build with specified prefix.
    fn build_with_key(self, env: &impl Environment) -> Result<Self::Product, PropertyError>;
}
