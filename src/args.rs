//! Provide arguments property source.
use crate::*;
use clap::{App, Arg};
use regex::Regex;
use std::collections::HashMap;

const NOT_POSSIBLE: &'static str = "Not possible";

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

    pub fn parse_args() -> Vec<(String, Property)> {
        let matches = App::new(env!("CARGO_PKG_NAME"))
            .version(env!("CARGO_PKG_VERSION"))
            .author(env!("CARGO_PKG_AUTHORS"))
            .about(env!("CARGO_PKG_DESCRIPTION"))
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
        Self::new(SysArgs::parse_args())
    }
}
