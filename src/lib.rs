//! A layered configuration loader with zero-boilerplate configuration management.
//!
//! ## About
//! `salak` is a rust version of layered configuration loader inspired by
//! [spring-boot](https://docs.spring.io/spring-boot/docs/current/reference/html/spring-boot-features.html#boot-features-external-config).
//! `salak` also has a [haskell version](https://hackage.haskell.org/package/salak).
//!
//! `salak` defines following default [`PropertySource`]s:
//! 1. Command line arguments using `clap` to parsing `-P, --propery KEY=VALUE`.
//! 2. System Environment.
//! 3. app.toml(*) in current dir and $HOME dir. Or if you specify `APP_CONF_DIR` dir, then only load toml in this dir.
//!
//! \* `APP_CONF_NAME` can be specified to replace `app`.
//!
//! ### Placeholder format
//! 1. `${key:default}` means get value of `key`, if not exists then return `default`.
//! 2. `${key}` means get value of `key`, if not exists then return `PropertyError::NotFound(_)`.
//! 3. `\$\{key\}` means escape to `${key}` or u can use `disable_placeholder` attribute.
//!
//! ### Key format
//! 1. `a.b.c` is a normal key separated by dot(`.`).
//! 2. `a.b[0]`, `a.b[1]`, `a.b[2]`... is a group of keys with arrays.
//! 3. System environment key will be changed from `HELLO_WORLD` <=> `hello.world`, `HELLO__WORLD_HOW` <=> `hello_world.how`, `hello[1].world` => `HELLO_1_WORLD` <=> `hello.1.world`.
//!
//! ### Auto derived parameters.
//!
//! ##### attribute `default` to set default value.
//! 1. `#[salak(default="string")]`
//! 2. `#[salak(default=1)]`
//!
//! ##### attribute `disable_placeholder` to disable placeholder parsing.
//! 1. `#[salak(disable_placeholder)]`
//! 2. `#[salak(disable_placeholder = true)]`
//!
//! ### Features
//!
//! ##### Default features
//! 1. `enable_log`, enable log record if enabled.
//! 2. `enable_toml`, enable toml support.
//! 3. `enable_derive`, enable auto derive [`FromEnvironment`] for struts.
//!
//! ##### Optional features
//! 1. `enable_clap`, enable default command line arguments parsing by `clap`.
//! 2. `enable_yaml`, enable yaml support.
//!
//! ## Quick Example
//!
//! ```
//! use salak::*;
//! #[derive(FromEnvironment, Debug)]
//! pub struct DatabaseConfig {
//!     url: String,
//!     #[salak(default = "salak")]
//!     username: String,
//!     password: Option<String>,
//!     description: String,
//! }
//!
//! std::env::set_var("database.url", "localhost:5432");
//! std::env::set_var("database.description", "\\$\\{Hello\\}");
//! let env = Salak::new()
//!    .with_default_args(auto_read_sys_args_param!()) // This line need enable feature `enable_clap`.
//!    .build();
//!
//! match env.require::<DatabaseConfig>("database") {
//!     Ok(val) => println!("{:?}", val),
//!     Err(e) => println!("{}", e),
//! }
//!
//! // Output: DatabaseConfig {
//! //  url: "localhost:5432",
//! //  name: "salak",
//! //  username: "salak",
//! //  password: None,
//! //  description: "${Hello}"
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
use std::collections::HashSet;
use std::hash::BuildHasher;
use std::hash::Hash;

#[cfg(test)]
#[macro_use(quickcheck)]
extern crate quickcheck_macros;

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

pub use crate::err::PropertyError;
pub use crate::utils::SalakStringUtil;

mod env;
pub use crate::env::{
    placeholder::PlaceholderResolver,
    registry::SourceRegistry,
    salak::{Salak, SalakBuilder},
};

mod source;

#[cfg(feature = "enable_toml")]
#[cfg_attr(docsrs, doc(cfg(feature = "enable_toml")))]
pub use crate::source::toml::Toml;
#[cfg(feature = "enable_yaml")]
#[cfg_attr(docsrs, doc(cfg(feature = "enable_yaml")))]
pub use crate::source::yaml::Yaml;
pub use crate::source::{args::*, env::SysEnvPropertySource, map::MapPropertySource};

/// Unified property structure.
#[derive(Clone, Debug)]
#[allow(missing_docs)]
pub enum Property {
    // String
    Str(String),
    // Integer
    Int(i64),
    // Float
    Float(f64),
    // Bool
    Bool(bool),
}

/// Convert to [`Property`].
pub trait IntoProperty: Sized {
    /// Convert to property.
    fn into_property(self) -> Property;
}

/// Convert value from [`Property`].
pub trait FromProperty: Sized {
    /// Convert from property.
    fn from_property(_: Property) -> Result<Self, PropertyError>;
}

/// An abstract source loader from various sources,
/// such as commandline arguments, system environment, files, etc.
pub trait PropertySource: Sync + Send {
    /// Name
    fn name(&self) -> String;
    /// Get property with name.
    fn get_property(&self, name: &str) -> Option<Property>;
    /// Check if property with name exists.
    fn contains_property(&self, name: &str) -> bool {
        self.get_property(name).is_some()
    }
    /// Check if the source is empty.
    fn is_empty(&self) -> bool;

    /// Find keys with prefix.
    fn get_keys(&self, prefix: &str) -> Vec<String>;

    /// Reload [`PropertySource`]
    fn load(&self) -> Result<Option<Box<dyn PropertySource>>, PropertyError>;
}

/// An environment for getting properties in multiple [`PropertySource`]s.
pub trait Environment: Sync + Send + Sized {
    /// Check if the environment has property.
    fn contains(&self, name: &str) -> bool {
        self.require::<Property>(name).is_ok()
    }
    /// Get required value, or return error.
    fn require<T: FromEnvironment>(&self, name: &str) -> Result<T, PropertyError>;

    /// Get required value, if not exists then return default value, otherwise return error.
    fn require_or<T: FromEnvironment>(&self, name: &str, default: T) -> Result<T, PropertyError> {
        match self.require::<Option<T>>(name) {
            Ok(Some(a)) => Ok(a),
            Ok(None) => Ok(default),
            Err(e) => Err(e),
        }
    }

    /// Get optional value, this function will ignore property parse error.
    fn get<T: FromEnvironment>(&self, name: &str) -> Option<T> {
        self.require(name).ok()
    }
    /// Get value or using default, this function will ignore property parse error.
    fn get_or<T: FromEnvironment>(&self, name: &str, default: T) -> T {
        self.get(name).unwrap_or(default)
    }

    /// Resolve placeholder.
    fn resolve_placeholder(&self, value: String) -> Result<Option<Property>, PropertyError>;

    /// Load properties
    #[cfg(feature = "enable_derive")]
    #[cfg_attr(docsrs, doc(cfg(feature = "enable_derive")))]
    fn load_config<T: DefaultSourceFromEnvironment>(&self) -> Result<T, PropertyError> {
        self.require(T::prefix())
    }

    /// Find keys with prefix.
    fn find_keys(&self, prefix: &str) -> Vec<String>;

    /// Reload configuration
    fn reload(&mut self) -> Result<(), PropertyError>;
}

/// Generate object from [`Environment`].
pub trait FromEnvironment: Sized {
    /// Generate object from env.
    fn from_env(
        prefix: &str,
        p: Option<Property>,
        env: &impl Environment,
    ) -> Result<Self, PropertyError>;

    /// Handle special case such as property not found.
    fn from_err(err: PropertyError) -> Result<Self, PropertyError> {
        Err(err)
    }

    /// Notify if the value is empty value. Such as `Vec<T>` or `Option<T>`.
    fn check_is_empty(&self) -> bool {
        false
    }

    /// Load default value.
    fn load_default() -> Vec<(String, Property)> {
        vec![]
    }
}

#[cfg(feature = "enable_toml")]
#[cfg(feature = "enable_derive")]
#[cfg(test)]
mod tests {
    use crate::*;
    use std::collections::HashMap;
    #[derive(FromEnvironment, Debug)]
    struct DatabaseConfigObj {
        hello: String,
        world: Option<String>,
    }
    #[derive(FromEnvironment, Debug)]
    struct DatabaseConfigDetail {
        #[salak(default = "str")]
        option_str: String,
        #[salak(default = 1)]
        option_i64: i64,
        option_arr: Vec<i64>,
        option_multi_arr: Vec<Vec<i64>>,
        option_obj: Vec<DatabaseConfigObj>,
    }

    #[derive(FromEnvironment, Debug)]
    #[salak(prefix = "database")]
    struct DatabaseConfig {
        url: String,
        name: String,
        #[salak(default = "${database.name}")]
        username: String,
        password: Option<String>,
        description: String,
        detail: DatabaseConfigDetail,
    }
    #[test]
    fn integration_tests() {
        let env = Salak::new()
            .with_custom_args(vec![
                ("database.detail.option_arr[0]".to_owned(), "10"),
                ("database.url".to_owned(), "localhost:5432"),
                ("database.name".to_owned(), "salak"),
                ("database.description".to_owned(), "\\$\\{Hello\\}"),
            ])
            .build();

        let ret = env.load_config::<DatabaseConfig>();
        assert_eq!(true, ret.is_ok());
        let ret = ret.unwrap();
        assert_eq!("localhost:5432", ret.url);
        assert_eq!("salak", ret.name);
        assert_eq!("salak", ret.username);
        assert_eq!(None, ret.password);
        assert_eq!("${Hello}", ret.description);
        let ret = ret.detail;
        assert_eq!("str", ret.option_str);
        assert_eq!(1, ret.option_i64);
        assert_eq!(5, ret.option_arr.len());
        assert_eq!(10, ret.option_arr[0]);
        assert_eq!(0, ret.option_multi_arr.len());
        assert_eq!(2, ret.option_obj.len());

        let ret = env.require::<HashMap<String, String>>("database");
        assert_eq!(true, ret.is_ok());
        let ret = ret.unwrap();
        assert_eq!(3, ret.len());
    }
}
