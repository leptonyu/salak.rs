//! Provide command line arguments [`PropertySource`].
use crate::map::MapPropertySource;
use crate::*;
use clap::{App, Arg};
use regex::Regex;
use std::collections::HashMap;

const NOT_POSSIBLE: &'static str = "Not possible";

/// CommandLine arguments parser mode.
pub enum SysArgsMode {
    /// Use default parser.
    Auto(SysArgsParam),
    /// Customize parser and provide a key value vector as [`PropertySource`].
    Custom(Vec<(String, Property)>),
}

/// Command line arguments parameters.
pub struct SysArgsParam {
    /// App name.
    pub name: &'static str,
    /// App version.
    pub version: &'static str,
    /// App authors.
    pub author: Option<&'static str>,
    /// App description.
    pub about: Option<&'static str>,
}

/// Auto generate [`SysArgsParam`] from Cargo.toml.
///
/// Due to macro [`env!`] will generate value at compile time, so users should call it at final project.
#[macro_export]
macro_rules! auto_read_sys_args_param {
    () => {
        args::SysArgsParam {
            name: env!("CARGO_PKG_NAME"),
            version: env!("CARGO_PKG_VERSION"),
            author: option_env!("CARGO_PKG_AUTHORS"),
            about: option_env!("CARGO_PKG_DESCRIPTION"),
        }
    };
}

/// A simple implementation of [`PropertySource`].
pub(crate) struct SysArgs(pub(crate) MapPropertySource);

impl SysArgs {
    /// Create [`SysArgs`].
    pub fn new(args: SysArgsMode) -> Self {
        let args = match args {
            SysArgsMode::Auto(arg) => Self::new_default_args(arg),
            SysArgsMode::Custom(arg) => arg,
        };

        let mut map = HashMap::new();
        for (k, v) in args {
            map.insert(k, v);
        }
        SysArgs(MapPropertySource::new("SystemArguments".to_owned(), map))
    }

    fn new_default_args(param: SysArgsParam) -> Vec<(String, Property)> {
        let mut app = App::new(param.name).version(param.version);
        if let Some(a) = param.author {
            app = app.author(a);
        }
        if let Some(a) = param.about {
            app = app.about(a);
        }
        let matches = app
            .arg(
                Arg::with_name("property")
                    .short("P")
                    .long("property")
                    .value_name("KEY=VALUE")
                    .multiple(true)
                    .number_of_values(1)
                    .takes_value(true)
                    .help("Set properties"),
            )
            .get_matches();
        lazy_static::lazy_static! {
            static ref RE: Regex = Regex::new(
                r"^([^=]+)=(.+)$"
            )
            .expect(NOT_POSSIBLE);
        }
        matches
            .values_of_lossy("property")
            .unwrap_or(vec![])
            .iter()
            .flat_map(|k| match RE.captures(&k) {
                Some(ref v) => Some((
                    v.get(1).unwrap().as_str().to_owned(),
                    Property::Str(v.get(2).unwrap().as_str().to_owned()),
                )),
                _ => None,
            })
            .collect()
    }
}

impl Default for SysArgs {
    /// A simple implementation using `clap`.
    fn default() -> Self {
        SysArgs::new(SysArgsMode::Auto(auto_read_sys_args_param!()))
    }
}
