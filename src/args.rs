use crate::*;
use clap::{App, Arg};
use std::collections::HashMap;

pub struct SysArgs {
    map: HashMap<String, Property>,
}

impl SysArgs {
    pub fn new(args: Vec<(String, Property)>) -> Self {
        let mut map = HashMap::new();
        for (k, v) in args {
            map.insert(k, v);
        }
        SysArgs { map }
    }
}

impl Default for SysArgs {
    fn default() -> Self {
        let matches = App::new(env!("CARGO_PKG_NAME"))
            .version(env!("CARGO_PKG_VERSION"))
            .author(env!("CARGO_PKG_AUTHORS"))
            .about(env!("CARGO_PKG_DESCRIPTION"))
            .arg(
                Arg::new("property")
                    .short('P')
                    .long("property")
                    .value_name("KEY=VALUE")
                    .multiple(true)
                    .number_of_values(1)
                    .takes_value(true)
                    .about("Set properties"),
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

impl PropertySource for SysArgs {
    fn name(&self) -> &'static str {
        "SystemArguments"
    }
    fn contains_property(&self, name: &str) -> bool {
        self.map.contains_key(name)
    }
    fn get_property(&self, name: &str) -> Option<Property> {
        self.map.get(name).map(|p| p.clone())
    }
}
