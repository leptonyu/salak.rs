use std::collections::HashSet;

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
    ResolveFail,
    RecursiveFail(String),
}

pub trait IsProperty: Sized {
    fn to_property(&self) -> Property<'_>;

    fn from_property(_: Property<'_>) -> Result<Self, PropertyError>;
}

impl ToString for Property<'_> {
    fn to_string(&self) -> String {
        match self {
            Property::S(v) => v.to_string(),
            Property::L(v) => v.to_string(),
            Property::I(v) => v.to_string(),
            Property::F(v) => v.to_string(),
            Property::B(v) => v.to_string(),
        }
    }
}

impl IsProperty for String {
    fn to_property(&self) -> Property<'_> {
        Property::S(self)
    }

    fn from_property(p: Property<'_>) -> Result<Self, PropertyError> {
        Ok(p.to_string())
    }
}

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
        match self.get_property(key).or(def) {
            Some(Property::S(v)) => {
                let mut history = HashSet::new();
                history.insert(key.to_string());
                Ok(Some(self.resolve(v, &mut history)?))
            }
            Some(Property::L(v)) => {
                let mut history = HashSet::new();
                history.insert(key.to_string());
                Ok(Some(self.resolve(&v[..], &mut history)?))
            }
            v => Ok(v),
        }
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
                    let v = if let Some(p) = self.get_property(key) {
                        String::from_property(p)?
                    } else if let Some(d) = def {
                        d.to_owned()
                    } else {
                        return Err(PropertyError::ResolveFail);
                    };
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
                .insert("j", "${${g}:a}"),
        );

        fn validate(env: &Registry, key: &str) {
            println!("{}: {:?}", key, env.get(key, None));
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
        validate(&env, "z");
    }
}
