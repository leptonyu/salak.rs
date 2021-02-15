//! Provide arguments property source.
use crate::*;
use clap::{App, Arg};
use regex::Regex;
use std::collections::HashMap;

const NOT_POSSIBLE: &'static str = "Not possible";

pub struct SysArgsParam {
    pub name: &'static str,
    pub version: &'static str,
    pub author: Option<&'static str>,
    pub about: Option<&'static str>,
}

/// Parse `SysArgsParam` from Cargo.toml.
#[macro_export]
macro_rules! sys_args_param {
    () => {
        args::SysArgsParam {
            name: env!("CARGO_PKG_NAME"),
            version: env!("CARGO_PKG_VERSION"),
            author: option_env!("CARGO_PKG_AUTHORS"),
            about: option_env!("CARGO_PKG_DESCRIPTION"),
        }
    };
}

/// A simple implementation of `PropertySource`.
pub struct SysArgs(pub(crate) MapPropertySource);

impl SysArgs {
    /// Create `SysArgs` from vec.
    pub fn new(args: Vec<(String, Property)>) -> Self {
        let mut map = HashMap::new();
        for (k, v) in args {
            map.insert(k, v);
        }
        SysArgs(MapPropertySource::new("SystemArguments".to_owned(), map))
    }

    /// Create `SysArgs` with default parser.
    pub fn new_default_args(param: SysArgsParam) -> Self {
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
        Self::new(
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
                .collect(),
        )
    }
}

impl Default for SysArgs {
    /// A simple implementation using `clap`.
    fn default() -> Self {
        SysArgs::new_default_args(sys_args_param!())
    }
}
