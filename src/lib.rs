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

const NOT_POSSIBLE: &'static str = "Not possible";

#[derive(Debug)]
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

pub trait Environment: Sized {
    fn contains(&self, name: &str) -> bool {
        self.require::<Property>(name).is_ok()
    }
    fn require<T: FromProperty>(&self, name: &str) -> Result<T, PropertyError>;
    fn get<T: FromProperty>(&self, name: &str) -> Option<T> {
        self.require(name).ok()
    }
}

pub struct PlaceHolderEnvironment<T: Environment> {
    env: T,
    placeholder: &'static [char],
}

impl<E: Environment> PlaceHolderEnvironment<E> {
    pub fn new(env: E) -> Self {
        PlaceHolderEnvironment {
            env,
            placeholder: &['{', '}'],
        }
    }

    fn parse(&self, mut val: &str) -> Result<String, PropertyError> {
        let mut stack: Vec<String> = vec![];
        let mut pre = "".to_owned();
        while let Some(left) = val.find(self.placeholder) {
            match &val[left..=left] {
                "{" => {
                    if stack.is_empty() {
                        pre.push_str(&val[..left]);
                        stack.push("".to_owned());
                    } else {
                        stack.push(val[..left].to_string());
                    }
                }
                _ => {
                    if let Some(mut name) = stack.pop() {
                        name.push_str(&val[..left]);
                        let mut def: Option<String> = None;
                        let key = if let Some(k) = name.find(':') {
                            def = Some(name[k + 1..].to_owned());
                            &name[..k]
                        } else {
                            &name
                        };
                        let value: String = self.require(&key).or_else(|e| def.ok_or(e))?;
                        if let Some(mut prefix) = stack.pop() {
                            prefix.push_str(&value);
                            stack.push(prefix);
                        } else {
                            pre.push_str(&value);
                        }
                    } else {
                        return Err(PropertyError::ParseFail(format!("Suffix not match 1")));
                    }
                }
            }
            val = &val[left + 1..];
        }
        if !stack.is_empty() {
            return Err(PropertyError::ParseFail(format!("Suffix not match 2")));
        }
        pre.push_str(&val);
        Ok(pre)
    }
}

impl<E: Environment> Environment for PlaceHolderEnvironment<E> {
    fn contains(&self, name: &str) -> bool {
        self.env.contains(name)
    }

    fn require<T>(&self, name: &str) -> Result<T, PropertyError>
    where
        T: FromProperty,
    {
        match self.env.require(name)? {
            Property::Str(s) => T::from_property(Property::Str(self.parse(&s)?)),
            p => T::from_property(p),
        }
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
        #[cfg(not(test))]
        sr.register_source(Box::new(SysArgs::default()));
        sr.register_source(Box::new(SysEnv));
        sr
    }
}

impl Environment for SourceRegistry {
    fn contains(&self, name: &str) -> bool {
        self.sources.iter().any(|a| a.contains_property(name))
    }
    fn require<T: FromProperty>(&self, name: &str) -> Result<T, PropertyError> {
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

#[cfg(test)]
mod tests {

    use crate::*;

    #[test]
    fn check() {
        std::env::set_var("v1", "value");
        std::env::set_var("v2", "{v1}");
        std::env::set_var("v3", "{no_found:default}");
        std::env::set_var("v4", "{no_found:{v2}}");
        std::env::set_var("v5", "{no_found:{no_found_2:hello}}");
        std::env::set_var("v6", "hello-{v1}-{v3}-");
        let env = PlaceHolderEnvironment::new(SourceRegistry::default());
        assert_eq!("value", &env.require::<String>("v1").unwrap());
        assert_eq!("value", &env.require::<String>("v2").unwrap());
        assert_eq!("default", &env.require::<String>("v3").unwrap());
        assert_eq!("value", &env.require::<String>("v4").unwrap());
        assert_eq!("hello", &env.require::<String>("v5").unwrap());
        assert_eq!(
            "hello-value-default-",
            &env.require::<String>("v6").unwrap()
        );
    }
}
