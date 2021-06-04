//! A resource initialization factory using `salak`.
//! `salak` is a zero-boilerplate configuration parser, it can
//! parsing struct from a unified [`Environment`]. After
//! we got the config struct, we can continue to initialize
//! resource from it. That means we have a unified way to
//! package the initialization process of resources, by specifying
//! configuration properties, and provide a customizer to customize
//! resource by coding.
//!
//!
//! ### Provide Resources
//! 1. redis
//! ```no_run
//! use salak::*;
//! use salak_factory::*;
//! use salak_factory::redis_default::*;
//! let env = Salak::new().unwrap();
//! let redis_pool = env.init_resource::<RedisPool>().unwrap();
//! ```
//! 2. redis_cluster
//! ```no_run
//! use salak::*;
//! use salak_factory::*;
//! use salak_factory::redis_cluster::*;
//! let env = Salak::new().unwrap();
//! let redis_cluster_pool = env.init_resource::<RedisPool>().unwrap();
//! ```
//! 3. postgres
//! ```no_run
//! use salak::*;
//! use salak_factory::*;
//! use salak_factory::postgresql::*;
//! let env = Salak::new().unwrap();
//! let pg_pool = env.init_resource::<PostgresPool>().unwrap();
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

#[allow(unused_imports)]
use salak::*;

#[macro_use]
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

/// Wrap enum for implement [`EnumProperty`].
#[derive(Debug)]
pub struct WrapEnum<T>(pub(crate) T);
