use std::sync::Mutex;
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    vec,
};

use crate::{
    Environment, FromEnvironment, IORef, IORefT, IsProperty, Key, Property, PropertyError,
    PropertySource, SalakContext, SubKey, SubKeys, PREFIX,
};
#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
use crate::{KeyDesc, PrefixedFromEnvironment};

/// An in-memory source, which is a string to string hashmap.
#[derive(Debug)]
pub struct HashMapSource {
    name: String,
    map: HashMap<String, String>,
}

impl HashMapSource {
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

impl PropertySource for HashMapSource {
    fn name(&self) -> &str {
        &self.name
    }

    fn get_property(&self, key: &Key<'_>) -> Option<Property<'_>> {
        self.map.get(key.as_str()).map(|s| Property::S(s))
    }

    fn sub_keys<'a>(&'a self, prefix: &Key<'_>, sub_keys: &mut SubKeys<'a>) {
        for key in self.map.keys() {
            if let Some(k) = key.strip_prefix(prefix.as_str()) {
                let pos = k.find('.').unwrap_or_else(|| k.len());
                sub_keys.insert(&k[0..pos]);
            }
        }
    }

    fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

/// Create source from system environment.
pub fn system_environment() -> HashMapSource {
    HashMapSource {
        name: "SystemEnvironment".to_owned(),
        map: std::env::vars().collect(),
    }
}

enum PS<'a> {
    Ref(&'a Box<dyn PropertySource>),
    Own(Box<dyn PropertySource>),
}
use core::ops::Deref;

impl Deref for PS<'_> {
    type Target = dyn PropertySource;

    fn deref(&self) -> &Self::Target {
        match self {
            PS::Own(f) => f.as_ref(),
            PS::Ref(f) => f.as_ref(),
        }
    }
}

pub(crate) struct PropertyRegistryInternal<'a> {
    name: &'a str,
    providers: Vec<PS<'a>>,
}

impl PropertySource for PropertyRegistryInternal<'_> {
    fn name(&self) -> &str {
        self.name
    }

    fn get_property(&self, key: &Key<'_>) -> Option<Property<'_>> {
        self.providers.iter().find_map(|p| p.get_property(key))
    }

    fn is_empty(&self) -> bool {
        self.providers.is_empty() || self.providers.iter().all(|f| f.is_empty())
    }

    fn sub_keys<'a>(&'a self, key: &Key<'_>, sub_keys: &mut SubKeys<'a>) {
        self.providers
            .iter()
            .for_each(|f| f.sub_keys(key, sub_keys));
    }
}

impl<'a> PropertyRegistryInternal<'a> {
    pub(crate) fn register_by_ref(&mut self, provider: Box<dyn PropertySource>) {
        if !provider.is_empty() {
            self.providers.push(PS::Own(provider));
        }
    }

    fn new(name: &'a str) -> Self {
        Self {
            name,
            providers: vec![],
        }
    }
}

/// An implementation of [`Environment`] for registering [`PropertySource`].
#[allow(missing_debug_implementations)]
pub struct PropertyRegistry<'a> {
    internal: PropertyRegistryInternal<'a>,
    reload: Mutex<Vec<Box<dyn IORefT + Send>>>,
}

impl<'a> Deref for PropertyRegistry<'a> {
    type Target = dyn PropertySource + 'a;

    fn deref(&self) -> &Self::Target {
        &self.internal
    }
}

impl Default for PropertyRegistry<'static> {
    fn default() -> Self {
        Self::new("registry")
    }
}

impl Environment for PropertyRegistry<'_> {
    fn reload(&self) -> Result<bool, PropertyError> {
        let mut flag = false;
        let internal = PropertyRegistryInternal {
            name: "reload",
            providers: self
                .internal
                .providers
                .iter()
                .map(|f| match f.reload_source() {
                    Ok(None) => Ok(match f {
                        PS::Own(v) => PS::Ref(&*v),
                        PS::Ref(v) => PS::Ref(*v),
                    }),
                    Ok(Some(v)) => {
                        flag = true;
                        Ok(PS::Own(v))
                    }
                    Err(err) => Err(err),
                })
                .collect::<Result<Vec<PS<'_>>, PropertyError>>()?,
        };

        let env = PropertyRegistry {
            internal,
            reload: Mutex::new(vec![]),
        };

        let guard = self.reload.lock().unwrap();
        for io in guard.iter() {
            io.reload_ref(&env)?;
        }
        Ok(flag)
    }

    fn require<T: FromEnvironment>(&self, key: &str) -> Result<T, PropertyError> {
        self.require_def(&mut Key::new(), SubKey::S(key), None)
    }
}

impl PropertyRegistry<'_> {
    #[cfg(feature = "derive")]
    /// Get key description.
    #[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
    pub(crate) fn get_desc<T: PrefixedFromEnvironment>(&self) -> Vec<KeyDesc> {
        let mut keys = vec![];
        self.key_desc::<T, &str>(&mut Key::new(), T::prefix(), None, None, None, &mut keys);
        keys
    }
}

impl<'a> SalakContext<'a> for PropertyRegistry<'a> {
    /// Parse property from env.
    fn require_def<T: FromEnvironment, K: Into<SubKey<'a>>>(
        &'a self,
        key: &mut Key<'a>,
        sub_key: K,
        def: Option<Property<'_>>,
    ) -> Result<T, PropertyError> {
        key.push(sub_key.into());
        let val = self.get(key, def).map(|val| T::from_env(key, val, self));
        key.pop();
        match val? {
            Err(PropertyError::ParseFail(None, v)) if !key.as_str().is_empty() => {
                Err(PropertyError::ParseFail(Some(key.as_str().to_string()), v))
            }
            val => val,
        }
    }

    #[cfg(feature = "derive")]
    fn key_desc<T: FromEnvironment, K: Into<SubKey<'a>>>(
        &'a self,
        key: &mut Key<'a>,
        sub_key: K,
        required: Option<bool>,
        def: Option<&'a str>,
        desc: Option<String>,
        keys: &mut Vec<KeyDesc>,
    ) {
        key.push(sub_key.into());
        let mut desc = KeyDesc::new(
            key.as_generic(),
            std::any::type_name::<T>(),
            required,
            def,
            desc,
        );
        T::key_desc(key, &mut desc, keys, self);
        if !desc.ignore {
            keys.push(desc);
        }
        key.pop();
    }

    fn register_ioref<T: Clone + FromEnvironment + Send + 'static>(&self, ioref: &IORef<T>) {
        let mut guard = self.reload.lock().unwrap();
        let io = ioref.clone();
        guard.push(Box::new(io));
    }
    fn sub_keys(&'a self, prefix: &Key<'_>, sub_keys: &mut SubKeys<'a>) {
        self.internal.sub_keys(prefix, sub_keys)
    }
}

impl<T: FromEnvironment> FromEnvironment for Option<T> {
    fn from_env<'a>(
        key: &mut Key<'a>,
        val: Option<Property<'_>>,
        env: &'a impl SalakContext<'a>,
    ) -> Result<Self, PropertyError> {
        match T::from_env(key, val, env) {
            Ok(v) => Ok(Some(v)),
            Err(PropertyError::NotFound(_)) => Ok(None),
            Err(err) => Err(err),
        }
    }

    #[cfg(feature = "derive")]
    #[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
    fn key_desc<'a>(
        key: &mut Key<'a>,
        desc: &mut KeyDesc,
        keys: &mut Vec<KeyDesc>,
        env: &'a impl SalakContext<'a>,
    ) {
        desc.set_required(false);
        T::key_desc(key, desc, keys, env);
    }
}

impl PropertyRegistry<'_> {
    /// Create an empty source.
    pub fn new(name: &'static str) -> Self {
        Self {
            internal: PropertyRegistryInternal {
                name,
                providers: vec![],
            },
            reload: Mutex::new(vec![]),
        }
    }

    pub(crate) fn register_by_ref(&mut self, provider: Box<dyn PropertySource + Send + Sync>) {
        if !provider.is_empty() {
            self.internal.providers.push(PS::Own(provider));
        }
    }

    /// Register source to registry, sources that register earlier will have higher priority of
    /// configuration.
    pub fn register<P: PropertySource + Send + Sync + 'static>(mut self, provider: P) -> Self {
        self.register_by_ref(Box::new(provider));
        self
    }

    pub(crate) fn get<'a>(
        &'a self,
        key: &mut Key<'_>,
        def: Option<Property<'a>>,
    ) -> Result<Option<Property<'a>>, PropertyError> {
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
        history.insert(key.as_str().to_string());
        Ok(Some(self.resolve(key, v, &mut history)?))
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
        key: &Key<'_>,
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
                        return Err(PropertyError::ResolveFail(key.as_str().to_string()));
                    }
                    let last = stack.pop();
                    stack.push(Self::merge(last, &val[..pos]));
                    stack.push("".to_owned());
                    val = &val[pos + 2..];
                }
                "\\" => {
                    let pos_1 = pos + 1;
                    if val.len() == pos_1 {
                        return Err(PropertyError::ResolveFail(key.as_str().to_string()));
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
                    let v = if let Some(p) = self.get(&mut Key::from_str(key), None)? {
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
                _ => return Err(PropertyError::ResolveFail(key.as_str().to_string())),
            }
        }
        if let Some(mut v) = stack.pop() {
            if stack.is_empty() {
                v.push_str(val);
                return Ok(Property::O(v));
            }
        }
        Err(PropertyError::ResolveFail(key.as_str().to_string()))
    }
}
pub(crate) struct FileConfig {
    dir: Option<String>,
    name: String,
    profile: String,
    env_profile: PropertyRegistryInternal<'static>,
    env_default: PropertyRegistryInternal<'static>,
}

impl FromEnvironment for FileConfig {
    fn from_env<'a>(
        key: &mut Key<'a>,
        _: Option<Property<'_>>,
        env: &'a impl SalakContext<'a>,
    ) -> Result<Self, PropertyError> {
        Ok(FileConfig {
            dir: env.require_def(key, SubKey::S("dir"), None)?,
            name: env.require_def(key, SubKey::S("filename"), Some(Property::S("app")))?,
            profile: env.require_def(key, SubKey::S("profile"), Some(Property::S("default")))?,
            env_profile: PropertyRegistryInternal::new("profile-files"),
            env_default: PropertyRegistryInternal::new("default-files"),
        })
    }

    #[cfg(feature = "derive")]
    #[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
    fn key_desc<'a>(
        key: &mut Key<'a>,
        _: &mut KeyDesc,
        keys: &mut Vec<KeyDesc>,
        env: &'a impl SalakContext<'a>,
    ) {
        env.key_desc::<Option<String>, &str>(key, "dir", None, None, None, keys);
        env.key_desc::<String, &str>(key, "filename", Some(false), Some("app"), None, keys);
        env.key_desc::<String, &str>(key, "profile", Some(false), Some("default"), None, keys);
    }
}

#[cfg(feature = "derive")]
impl PrefixedFromEnvironment for FileConfig {
    fn prefix() -> &'static str {
        PREFIX
    }
}

impl FileConfig {
    #[allow(dead_code)]
    pub(crate) fn new(env: &impl Environment) -> Result<Self, PropertyError> {
        env.require::<FileConfig>(PREFIX)
    }

    #[allow(dead_code)]
    pub(crate) fn register_to_env(self, env: &mut PropertyRegistry<'_>) {
        env.register_by_ref(Box::new(self.env_profile));
        env.register_by_ref(Box::new(self.env_default));
    }

    #[allow(dead_code)]
    pub(crate) fn build<
        F: Fn(String, &str) -> Result<S, PropertyError>,
        S: PropertySource + Send + Sync + 'static,
    >(
        &mut self,
        ext: &str,
        f: F,
    ) -> Result<(), PropertyError> {
        fn make<
            F: Fn(String, &str) -> Result<S, PropertyError>,
            S: PropertySource + Send + Sync + 'static,
        >(
            f: F,
            file: String,
            dir: &Option<String>,
            env: &mut PropertyRegistryInternal<'_>,
        ) -> Result<(), PropertyError> {
            let mut path = PathBuf::new();
            if let Some(d) = &dir {
                path.push(d);
            }
            path.push(file);
            if path.exists() {
                env.register_by_ref(Box::new((f)(
                    path.as_path().display().to_string(),
                    &std::fs::read_to_string(path)?,
                )?));
            }
            Ok(())
        }

        make(
            &f,
            format!("{}-{}.{}", self.name, self.profile, ext),
            &self.dir,
            &mut self.env_profile,
        )?;
        make(
            &f,
            format!("{}.{}", self.name, ext),
            &self.dir,
            &mut self.env_default,
        )
    }
}
