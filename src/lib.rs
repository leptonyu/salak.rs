use clap::{App, Arg};
use regex::Regex;
use std::collections::HashMap;
use std::fmt::{Display, Error, Formatter};

#[derive(Clone)]
pub enum Property {
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
}

pub enum PropertyError {
    ParseFail(String),
}

impl Display for PropertyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match self {
            PropertyError::ParseFail(e) => write!(f, "{}", e),
        }
    }
}

pub trait PropertySource {
    fn name(&self) -> &'static str;
    fn get_property(&self, name: &str) -> Option<Property>;
    fn contains_property(&self, name: &str) -> bool {
        self.get_property(name).is_some()
    }
}

pub struct SysEnv;

impl PropertySource for SysEnv {
    fn name(&self) -> &'static str {
        "SystemEnvironment"
    }
    fn get_property(&self, name: &str) -> Option<Property> {
        std::env::var(name).ok().map(|a| Property::Str(a))
    }
}

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
                Arg::with_name("property")
                    .short("P")
                    .value_name("Key=Value")
                    .multiple(true)
                    .number_of_values(1)
                    .takes_value(true)
                    .help("Sets a custom config file"),
            )
            .get_matches();
        lazy_static::lazy_static! {
            static ref RE: Regex = Regex::new(
                r"^([^=]+)=(.+)$"
            )
            .expect("Not possible");
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
    fn get_property(&self, name: &str) -> Option<Property> {
        self.map.get(name).map(|p| p.clone())
    }
}

pub trait FromProperty: Sized {
    fn from_property(_: Property) -> Result<Self, PropertyError>;
}

impl FromProperty for Property {
    fn from_property(a: Property) -> Result<Self, PropertyError> {
        Ok(a)
    }
}

impl FromProperty for String {
    fn from_property(p: Property) -> Result<Self, PropertyError> {
        match p {
            Property::Str(str) => Ok(str),
            Property::Int(i64) => Ok(i64.to_string()),
            Property::Float(f64) => Ok(f64.to_string()),
            Property::Bool(bool) => Ok(bool.to_string()),
        }
    }
}

impl FromProperty for i64 {
    fn from_property(p: Property) -> Result<Self, PropertyError> {
        match p {
            Property::Str(str) => str
                .parse::<i64>()
                .map_err(|e| PropertyError::ParseFail(e.to_string())),
            Property::Int(i64) => Ok(i64),
            Property::Float(f64) => Ok(f64 as i64),
            Property::Bool(_) => Err(PropertyError::ParseFail(
                "Bool value cannot convert to i64".to_owned(),
            )),
        }
    }
}

impl FromProperty for f64 {
    fn from_property(p: Property) -> Result<Self, PropertyError> {
        match p {
            Property::Str(str) => str
                .parse::<f64>()
                .map_err(|e| PropertyError::ParseFail(e.to_string())),
            Property::Int(i64) => Ok(i64 as f64),
            Property::Float(f64) => Ok(f64),
            Property::Bool(_) => Err(PropertyError::ParseFail(
                "Bool value cannot convert to f64".to_owned(),
            )),
        }
    }
}

impl FromProperty for bool {
    fn from_property(p: Property) -> Result<Self, PropertyError> {
        match p {
            Property::Str(str) => Ok(str.to_lowercase() == "true"),
            Property::Int(i64) => Ok(i64 == 0),
            Property::Float(f64) => Ok(f64 == 0.0),
            Property::Bool(bool) => Ok(bool),
        }
    }
}

pub trait Environment {
    fn contains(&self, name: &str) -> bool {
        self.required::<Property>(name).is_ok()
    }
    fn required<T: FromProperty>(&self, name: &str) -> Result<T, PropertyError>;
    fn get<T: FromProperty>(&self, name: &str) -> Option<T> {
        self.required(name).ok()
    }
}

pub struct SourceRegistry {
    sources: Vec<Box<dyn PropertySource>>,
}

impl SourceRegistry {
    pub fn new() -> Self {
        SourceRegistry { sources: vec![] }
    }

    pub fn register_source(&mut self, source: Box<dyn PropertySource>) {
        self.sources.push(source);
    }
}

impl Default for SourceRegistry {
    fn default() -> Self {
        let mut sr = Self::new();
        sr.register_source(Box::new(SysArgs::default()));
        sr.register_source(Box::new(SysEnv));
        sr
    }
}

impl Environment for SourceRegistry {
    fn required<T: FromProperty>(&self, name: &str) -> Result<T, PropertyError> {
        for ps in self.sources.iter() {
            if let Some(v) = ps.get_property(name) {
                return T::from_property(v);
            }
        }
        Err(PropertyError::ParseFail(format!(
            "Property {} not found",
            name
        )))
    }
}
