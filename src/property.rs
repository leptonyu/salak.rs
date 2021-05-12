use std::{collections::HashSet, error::Error};

#[derive(Debug)]
pub enum Property<'a> {
    S(&'a str),
    L(String),
    I(i64),
    F(f64),
    B(bool),
}

#[derive(Debug)]
pub enum PropertyError {
    ParseFail(Option<Box<dyn Error>>),
    ResolveFail,
    ResolveNotFound(String),
    RecursiveFail(String),
    NotFound(String),
}

impl<E: Error + 'static> From<E> for PropertyError {
    fn from(err: E) -> Self {
        PropertyError::ParseFail(Some(Box::new(err)))
    }
}

pub trait IsProperty: Sized {
    fn is_empty(p: &Property<'_>) -> bool {
        match p {
            Property::S(s) => s.is_empty(),
            Property::L(s) => s.is_empty(),
            _ => false,
        }
    }
    fn from_property(_: Property<'_>) -> Result<Self, PropertyError>;
}

impl IsProperty for String {
    fn is_empty(p: &Property<'_>) -> bool {
        false
    }
    fn from_property(p: Property<'_>) -> Result<Self, PropertyError> {
        Ok(match p {
            Property::S(v) => v.to_string(),
            Property::L(v) => v,
            Property::I(v) => v.to_string(),
            Property::F(v) => v.to_string(),
            Property::B(v) => v.to_string(),
        })
    }
}
impl IsProperty for bool {
    fn from_property(p: Property<'_>) -> Result<Self, PropertyError> {
        fn str_to_bool(v: &str) -> Result<bool, PropertyError> {
            match v {
                "yes" | "true" => Ok(true),
                "no" | "false" => Ok(false),
                _ => Err(PropertyError::ParseFail(None)),
            }
        }
        match p {
            Property::B(v) => Ok(v),
            Property::S(v) => str_to_bool(v),
            Property::L(v) => str_to_bool(&v),
            _ => Err(PropertyError::ParseFail(None)),
        }
    }
}

macro_rules! impl_property_num {
    ($($x:ident),+) => {$(
            impl IsProperty for $x {
                fn from_property(p: Property<'_>) -> Result<Self, PropertyError> {
                    Ok(match p {
                    Property::S(s) => s.parse::<$x>()?,
                    Property::L(s) => s.parse::<$x>()?,
                    Property::I(s) => s as $x,
                    Property::F(s) => s as $x,
                    _ => Err(PropertyError::ParseFail(None))?,
                    })
                }

            }

            )+}
}

impl_property_num!(i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, isize, usize);

pub trait PropertyProvider {
    fn name(&self) -> &'static str;

    fn get_property(&self, key: &str) -> Option<Property<'_>>;

    fn contains_key(&self, key: &str) -> bool {
        self.get_property(key).is_some()
    }

    fn is_empty(&self) -> bool;
}

pub struct Registry {
    providers: Vec<Box<dyn PropertyProvider>>,
}

impl PropertyProvider for Registry {
    fn name(&self) -> &'static str {
        "registry"
    }

    fn get_property(&self, key: &str) -> Option<Property<'_>> {
        self.providers.iter().find_map(|p| p.get_property(key))
    }

    fn contains_key(&self, key: &str) -> bool {
        self.providers.iter().any(|f| f.contains_key(key))
    }

    fn is_empty(&self) -> bool {
        self.providers.is_empty() || self.providers.iter().all(|f| f.is_empty())
    }
}

impl Registry {
    pub fn new() -> Self {
        Self { providers: vec![] }
    }

    pub fn register<P: PropertyProvider + 'static>(mut self, provider: P) -> Self {
        self.providers.push(Box::new(provider));
        self
    }

    pub fn get<'a>(
        &'a self,
        key: &str,
        def: Option<Property<'a>>,
    ) -> Result<Option<Property<'a>>, PropertyError> {
        let key = Self::normalize_key(key);
        let tmp;
        let v = match self.get_property(key).or(def) {
            Some(Property::S(v)) => v,
            Some(Property::L(v)) => {
                tmp = v;
                &tmp[..]
            }
            v => return Ok(v),
        };
        let mut history = HashSet::new();
        history.insert(key.to_string());
        Ok(Some(self.resolve(v, &mut history)?))
    }

    fn normalize_key(mut key: &str) -> &str {
        while !key.is_empty() && &key[0..1] == "." {
            key = &key[1..];
        }
        key
    }
    fn merge(val: Option<String>, new: &str) -> String {
        match val {
            Some(mut v) => {
                v.push_str(new);
                v
            }
            None => new.to_owned(),
        }
    }

    fn resolve(
        &self,
        mut val: &str,
        history: &mut HashSet<String>,
    ) -> Result<Property<'_>, PropertyError> {
        let mut stack = vec!["".to_owned()];
        let pat: &[_] = &['$', '\\', '}'];

        while let Some(pos) = val.find(pat) {
            match &val[pos..=pos] {
                "$" => {
                    let pos_1 = pos + 1;
                    if val.len() == pos_1 || &val[pos_1..=pos_1] != "{" {
                        return Err(PropertyError::ResolveFail);
                    }
                    let last = stack.pop();
                    stack.push(Self::merge(last, &val[..pos]));
                    stack.push("".to_owned());
                    val = &val[pos + 2..];
                }
                "\\" => {
                    let pos_1 = pos + 1;
                    if val.len() == pos_1 {
                        return Err(PropertyError::ResolveFail);
                    }
                    let last = stack.pop();
                    let mut v = Self::merge(last, &val[..pos]);
                    v.push_str(&val[pos_1..=pos_1]);
                    stack.push(v);
                    val = &val[pos + 2..];
                }
                "}" => {
                    let last = stack.pop();
                    let v = Self::merge(last, &val[..pos]);
                    let (key, def) = match v.find(":") {
                        Some(pos) => (&v[..pos], Some(&v[pos + 1..])),
                        _ => (&v[..], None),
                    };
                    if !history.insert(key.to_string()) {
                        return Err(PropertyError::RecursiveFail(key.to_owned()));
                    }
                    let v = if let Some(p) = self.get(key, None)? {
                        String::from_property(p)?
                    } else if let Some(d) = def {
                        d.to_owned()
                    } else {
                        return Err(PropertyError::ResolveNotFound(key.to_string()));
                    };
                    history.remove(key);
                    let v = Self::merge(stack.pop(), &v);
                    stack.push(v);
                    val = &val[pos + 1..];
                }
                _ => return Err(PropertyError::ResolveFail),
            }
        }
        if let Some(mut v) = stack.pop() {
            if stack.is_empty() {
                v.push_str(val);
                return Ok(Property::L(v));
            }
        }
        Err(PropertyError::ResolveFail)
    }
}

pub struct MapProvider {
    name: &'static str,
    map: std::collections::HashMap<String, String>,
}

impl MapProvider {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            map: std::collections::HashMap::new(),
        }
    }

    pub fn insert<K: Into<String>, V: Into<String>>(mut self, key: K, val: V) -> Self {
        self.map.insert(key.into(), val.into());
        self
    }
}

impl PropertyProvider for MapProvider {
    fn name(&self) -> &'static str {
        self.name
    }

    fn get_property(&self, key: &str) -> Option<Property<'_>> {
        self.map.get(key).map(|s| Property::S(s))
    }

    fn contains_key(&self, key: &str) -> bool {
        self.map.contains_key(key)
    }

    fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

pub fn system_environment() -> MapProvider {
    MapProvider {
        name: "SystemEnvironment",
        map: std::env::vars().collect(),
    }
}

pub trait Environment {
    fn require_def<T: FromEnvironment>(
        &self,
        key: &str,
        def: Option<Property<'_>>,
    ) -> Result<T, PropertyError>;
    fn require<T: FromEnvironment>(&self, key: &str) -> Result<T, PropertyError> {
        self.require_def(key, None)
    }
}

pub trait FromEnvironment: Sized {
    fn from_env(
        key: &str,
        val: Option<Property<'_>>,
        env: &impl Environment,
    ) -> Result<Self, PropertyError>;
}

impl<T: FromEnvironment> FromEnvironment for Option<T> {
    fn from_env(
        key: &str,
        val: Option<Property<'_>>,
        env: &impl Environment,
    ) -> Result<Self, PropertyError> {
        match T::from_env(key, val, env) {
            Ok(v) => Ok(Some(v)),
            Err(PropertyError::NotFound(_)) => Ok(None),
            Err(err) => Err(err),
        }
    }
}

impl<T: IsProperty> FromEnvironment for T {
    fn from_env(
        key: &str,
        val: Option<Property<'_>>,
        env: &impl Environment,
    ) -> Result<Self, PropertyError> {
        match val {
            Some(v) => Self::from_property(v),
            _ => Err(PropertyError::NotFound(
                Registry::normalize_key(key).to_string(),
            )),
        }
    }
}

impl Environment for Registry {
    fn require_def<T: FromEnvironment>(
        &self,
        key: &str,
        def: Option<Property<'_>>,
    ) -> Result<T, PropertyError> {
        T::from_env(key, self.get(key, def)?, self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn property_test() {
        let env = Registry::new().register(
            system_environment()
                .insert("a", "0")
                .insert("b", "${b}")
                .insert("c", "${a}")
                .insert("d", "${z}")
                .insert("e", "${z:}")
                .insert("f", "${z:${a}}")
                .insert("g", "a")
                .insert("h", "${${g}}")
                .insert("i", "\\$\\{a\\}")
                .insert("j", "${${g}:a}")
                .insert("k", "${a} ${a}")
                .insert("l", "${c}"),
        );

        fn validate(env: &Registry, key: &str) {
            println!("{}: {:?}", key, env.get(key, None));
            println!("{}: {:?}", key, env.require::<String>(key));
            println!("{}: {:?}", key, env.require::<bool>(key));
            println!("{}: {:?}", key, env.require::<u8>(key));
            println!("{}: {:?}", key, env.require::<Option<u8>>(key));
        }

        validate(&env, "a");
        validate(&env, "b");
        validate(&env, "c");
        validate(&env, "d");
        validate(&env, "e");
        validate(&env, "f");
        validate(&env, "g");
        validate(&env, "h");
        validate(&env, "i");
        validate(&env, "j");
        validate(&env, "k");
        validate(&env, "l");
        validate(&env, "z");
    }

    #[derive(Debug)]
    struct Config {
        i8: i8,
    }

    impl FromEnvironment for Config {
        fn from_env(
            key: &str,
            val: Option<Property<'_>>,
            env: &impl Environment,
        ) -> Result<Self, PropertyError> {
            Ok(Config {
                i8: env.require(&format!("{}.i8", key))?,
            })
        }
    }
    #[test]
    fn config_test() {
        let env = Registry::new().register(
            system_environment()
                .insert("a", "0")
                .insert("b", "${b}")
                .insert("c", "${a}")
                .insert("d", "${z}")
                .insert("e", "${z:}")
                .insert("f", "${z:${a}}")
                .insert("g", "a")
                .insert("h", "${${g}}")
                .insert("i", "\\$\\{a\\}")
                .insert("j", "${${g}:a}")
                .insert("k", "${a} ${a}")
                .insert("l", "${c}"),
        );

        println!("{:?}", env.require::<Config>(""));
        println!("{:?}", env.require::<Option<Config>>(""));
    }
}
