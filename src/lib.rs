use crate::property::*;
use regex::Regex;
use std::collections::HashSet;
use std::fmt::{Display, Error, Formatter};

pub mod args;
pub mod env;
pub mod property;

const NOT_POSSIBLE: &'static str = "Not possible";

#[derive(Clone)]
pub enum Property {
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
}

#[derive(Debug, PartialEq, Eq)]
pub enum PropertyError {
    ParseFail(String),
    RecursiveParse(String),
}

impl Display for PropertyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match self {
            PropertyError::ParseFail(e) => write!(f, "{}", e),
            PropertyError::RecursiveParse(n) => write!(f, "Recursive parsing property {}.", &n),
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

    fn do_parse<T: FromProperty>(
        &self,
        name: &str,
        contains: &mut HashSet<String>,
    ) -> Result<T, PropertyError> {
        if !contains.insert(name.to_owned()) {
            return Err(PropertyError::RecursiveParse(name.to_owned()));
        }
        match self.env.require(name)? {
            Property::Str(s) => T::from_property(Property::Str(self.parse(&s, contains)?)),
            p => T::from_property(p),
        }
    }

    fn parse(
        &self,
        mut val: &str,
        contains: &mut HashSet<String>,
    ) -> Result<String, PropertyError> {
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
                        let value: String =
                            self.do_parse(&key, contains).or_else(|e| def.ok_or(e))?;
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
        self.do_parse(name, &mut HashSet::new())
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
        sr.register_source(Box::new(args::SysArgs::default()));
        sr.register_source(Box::new(env::SysEnv));
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
        std::env::set_var("v7", "{v7}");
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

        let v7 = env.require::<String>("v7");

        assert_eq!(true, v7.is_err());
        assert_eq!(
            PropertyError::RecursiveParse("v7".to_string()),
            v7.unwrap_err()
        );
    }
}
