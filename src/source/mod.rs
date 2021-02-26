#[macro_use]
pub(crate) mod args;
pub(crate) mod env;
pub(crate) mod file;
pub(crate) mod internal;
pub(crate) mod map;
// Enable register toml in [`Environment`].
#[cfg(feature = "enable_toml")]
#[cfg_attr(docsrs, doc(cfg(feature = "enable_toml")))]
pub(crate) mod toml;
// Enable register yaml in [`Environment`].
#[cfg(feature = "enable_yaml")]
#[cfg_attr(docsrs, doc(cfg(feature = "enable_yaml")))]
pub(crate) mod yaml;
// use crate::*;
pub(crate) use file::*;
