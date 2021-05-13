use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    vec,
};

use crate::{
    normalize_key, Environment, FromEnvironment, IsProperty, Property, PropertyError,
    PropertySource,
};

/// An in-memory source, which is a string to string hashmap.
#[derive(Debug)]
pub struct MapProvider {
    name: String,
    map: HashMap<String, String>,
}

impl MapProvider {
    /// Create an in-memory source with a name.
    pub fn new(name: &'static str) -> Self {
        Self {
            name: name.to_owned(),
            map: HashMap::new(),
        }
    }

    /// Set property to the source.
    pub fn set<K: Into<String>, V: Into<String>>(mut self, key: K, val: V) -> Self {
        self.map.insert(key.into(), val.into());
        self
    }

    /// Set a batch of properties to the source.
    pub fn set_all(mut self, map: HashMap<String, String>) -> Self {
        self.map.extend(map);
        self
    }
}

impl PropertySource for MapProvider {
    fn name(&self) -> &str {
        &self.name
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

/// Create source from system environment.
pub fn system_environment() -> MapProvider {
    MapProvider {
        name: "SystemEnvironment".to_owned(),
        map: std::env::vars().collect(),
    }
}

/// An implementation of [`Environment`] for registering [`PropertySource`].
#[allow(missing_debug_implementations)]
pub struct PropertyRegistry {
    providers: Vec<Box<dyn PropertySource>>,
}

impl PropertySource for PropertyRegistry {
    fn name(&self) -> &str {
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

impl Default for PropertyRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PropertyRegistry {
    /// Create an empty source.
    pub fn new() -> Self {
        Self { providers: vec![] }
    }

    pub(crate) fn register_by_ref<P: PropertySource + 'static>(&mut self, provider: P) {
        if !provider.is_empty() {
            self.providers.push(Box::new(provider));
        }
    }

    /// Register source to registry, sources that register earlier will have higher priority of
    /// configuration.
    pub fn register<P: PropertySource + 'static>(mut self, provider: P) -> Self {
        self.register_by_ref(provider);
        self
    }

    pub(crate) fn get<'a>(
        &'a self,
        key: &str,
        def: Option<Property<'a>>,
    ) -> Result<Option<Property<'a>>, PropertyError> {
        let key = normalize_key(key);
        let tmp;
        let v = match self.get_property(key).or(def) {
            Some(Property::S(v)) => v,
            Some(Property::O(v)) => {
                tmp = v;
                &tmp[..]
            }
            v => return Ok(v),
        };
        let mut history = HashSet::new();
        history.insert(key.to_string());
        Ok(Some(self.resolve(v, &mut history)?))
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
                    let (key, def) = match v.find(':') {
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
                return Ok(Property::O(v));
            }
        }
        Err(PropertyError::ResolveFail)
    }
}

#[derive(Debug)]
pub(crate) struct FileConfig {
    dir: Option<String>,
    name: String,
    profile: String,
}

const PREFIX: &str = "application";

impl FromEnvironment for FileConfig {
    fn from_env(
        key: &str,
        _: Option<Property<'_>>,
        env: &impl Environment,
    ) -> Result<Self, PropertyError> {
        Ok(FileConfig {
            dir: env.require(&format!("{}.dir", key))?,
            name: env.require_def(&format!("{}.name", key), Some(Property::S("app")))?,
            profile: env.require_def(&format!("{}.profile", key), Some(Property::S("default")))?,
        })
    }
}

impl FileConfig {
    pub(crate) fn new(env: &impl Environment) -> Result<Self, PropertyError> {
        env.require::<FileConfig>(PREFIX)
    }

    pub(crate) fn build<F: Fn(String, &str) -> Result<S, PropertyError>, S: PropertySource>(
        &self,
        ext: &str,
        f: F,
    ) -> Result<Vec<S>, PropertyError> {
        let mut vs = vec![];
        for file in vec![
            format!("{}.{}", self.name, ext),
            format!("{}-{}.{}", self.name, self.profile, ext),
        ] {
            let mut path = PathBuf::new();
            if let Some(d) = &self.dir {
                path.push(d);
            }
            path.push(file);
            if path.exists() {
                vs.push((f)(
                    path.as_path().display().to_string(),
                    &std::fs::read_to_string(path)?,
                )?);
            }
        }
        Ok(vs)
    }
}
