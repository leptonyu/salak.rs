use core::ops::Deref;
use parking_lot::Mutex;
use std::{collections::HashSet, path::PathBuf, vec};

use crate::{
    wrapper::IORef, FromEnvironment, IORefT, IsProperty, Key, Property, PropertyError,
    PropertySource, SalakContext, SubKey, SubKeys, PREFIX,
};
#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
use crate::{DescFromEnvironment, KeyDesc, PrefixedFromEnvironment, SalakDescContext};
use crate::{Res, Void};

enum PS<'a> {
    Ref(&'a Box<dyn PropertySource>),
    Own(Box<dyn PropertySource>),
}

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

    #[inline]
    fn get_property(&self, key: &Key<'_>) -> Option<Property<'_>> {
        self.providers.iter().find_map(|p| p.get_property(key))
    }

    fn is_empty(&self) -> bool {
        self.providers.is_empty() || self.providers.iter().all(|f| f.is_empty())
    }

    fn get_sub_keys<'a>(&'a self, key: &Key<'_>, sub_keys: &mut SubKeys<'a>) {
        self.providers
            .iter()
            .for_each(|f| f.get_sub_keys(key, sub_keys));
    }
}

impl<'a> PropertyRegistryInternal<'a> {
    pub(crate) fn register_by_ref(&mut self, provider: Box<dyn PropertySource>) {
        if !provider.is_empty() {
            self.providers.push(PS::Own(provider));
        }
    }

    pub(crate) fn register<P: PropertySource + Send + Sync + 'static>(
        mut self,
        provider: P,
    ) -> Self {
        self.register_by_ref(Box::new(provider));
        self
    }

    pub(crate) fn new(name: &'a str) -> Self {
        Self {
            name,
            providers: vec![],
        }
    }

    fn get(
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

    #[inline]
    fn merge(val: Option<String>, new: &str) -> String {
        match val {
            Some(mut v) => {
                v.push_str(new);
                v
            }
            None => new.to_owned(),
        }
    }

    #[inline]
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

    pub(crate) fn reload(&self, iorefs: &'a Mutex<Vec<Box<dyn IORefT + Send>>>) -> Res<bool> {
        let mut flag = false;
        let registry = PropertyRegistryInternal {
            name: "reload",
            providers: self
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

        let guard = iorefs.lock();
        for io in guard.iter() {
            io.reload_ref(&registry, iorefs)?;
        }
        Ok(flag)
    }

    #[inline]
    pub(crate) fn require<T: FromEnvironment>(
        &self,
        sub_key: &str,
        iorefs: &'a Mutex<Vec<Box<dyn IORefT + Send>>>,
    ) -> Res<T> {
        let mut key = Key::new();
        SalakContext::new(&self, iorefs, &mut key).require_def(sub_key, None)
    }
}
#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
impl<'a> SalakDescContext<'a> {
    pub(crate) fn new(key: &'a mut Key<'a>, descs: &'a mut Vec<KeyDesc>) -> Self {
        let current = KeyDesc::new(key.as_str().to_string(), "String", None, None, None);
        Self {
            key,
            descs,
            current,
        }
    }

    /// Add key description.
    #[inline]
    pub fn add_key_desc<T: DescFromEnvironment>(
        &mut self,
        sub_key: &'a str,
        required: Option<bool>,
        def: Option<&'a str>,
        desc: Option<String>,
    ) {
        self.add_key_desc_internal::<T, &str>(sub_key, required, def, desc)
    }

    pub(crate) fn add_key_desc_internal<T: DescFromEnvironment, K: Into<SubKey<'a>>>(
        &mut self,
        sub_key: K,
        required: Option<bool>,
        def: Option<&'a str>,
        desc: Option<String>,
    ) {
        self.into_sub_key(sub_key);
        let key = self.key.as_generic();
        let bak = std::mem::replace(
            &mut self.current,
            KeyDesc::new(key, std::any::type_name::<T>(), required, def, desc),
        );
        T::key_desc(self);
        let desc = std::mem::replace(&mut self.current, bak);
        if !desc.ignore {
            self.descs.push(desc);
        }
        self.key.pop();
    }
    fn into_sub_key<K: Into<SubKey<'a>>>(&mut self, k: K) {
        self.key.push(k.into());
    }
}

impl<'a> SalakContext<'a> {
    /// Parse property from env.
    #[inline]
    pub fn require_def<T: FromEnvironment>(
        &mut self,
        sub_key: &'a str,
        def: Option<Property<'_>>,
    ) -> Res<T> {
        self.require_def_internal(sub_key, def)
    }

    #[inline]
    pub(crate) fn require_def_internal<T: FromEnvironment, K: Into<SubKey<'a>>>(
        &mut self,
        sub_key: K,
        def: Option<Property<'_>>,
    ) -> Res<T> {
        let flag = self.into_sub_key(sub_key);
        let val = match self.registry.get(self.key, def) {
            Ok(val) => Ok(T::from_env(val, self)),
            Err(e) => Err(e),
        };
        if flag {
            self.key.pop();
        }
        match val? {
            Err(PropertyError::ParseFail(None, v)) if !self.key.as_str().is_empty() => Err(
                PropertyError::ParseFail(Some(self.key.as_str().to_string()), v),
            ),
            val => val,
        }
    }

    pub(crate) fn get_sub_keys(&mut self) -> SubKeys<'a> {
        let mut sub_keys = SubKeys::new();
        self.registry.get_sub_keys(&mut self.key, &mut sub_keys);
        sub_keys
    }

    #[inline]
    pub(crate) fn current_key(&self) -> &str {
        self.key.as_str()
    }

    fn into_sub_key<K: Into<SubKey<'a>>>(&mut self, k: K) -> bool {
        let v = k.into();
        let flag = !v.is_empty();
        if flag {
            self.key.push(v);
        }
        return flag;
    }

    pub(crate) fn new(
        registry: &'a PropertyRegistryInternal<'a>,
        iorefs: &'a Mutex<Vec<Box<dyn IORefT + Send>>>,
        key: &'a mut Key<'a>,
    ) -> Self {
        Self {
            registry,
            key,
            iorefs,
        }
    }

    #[inline]
    pub(crate) fn register_ioref<T: Clone + FromEnvironment + Send + 'static>(
        &self,
        ioref: &IORef<T>,
    ) {
        let mut guard = self.iorefs.lock();
        let io = ioref.clone();
        guard.push(Box::new(io));
    }
}

impl<T: FromEnvironment> FromEnvironment for Option<T> {
    fn from_env(val: Option<Property<'_>>, env: &mut SalakContext<'_>) -> Res<Self> {
        match T::from_env(val, env) {
            Ok(v) => Ok(Some(v)),
            Err(PropertyError::NotFound(_)) => Ok(None),
            Err(err) => Err(err),
        }
    }
}

#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
impl<T: DescFromEnvironment> DescFromEnvironment for Option<T> {
    fn key_desc(env: &mut SalakDescContext<'_>) {
        env.current.set_required(false);
        T::key_desc(env);
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
    fn from_env(_: Option<Property<'_>>, env: &mut SalakContext<'_>) -> Res<Self> {
        Ok(FileConfig {
            dir: env.require_def("dir", None)?,
            name: env.require_def("filename", Some(Property::S("app")))?,
            profile: env.require_def("profile", Some(Property::S("default")))?,
            env_profile: PropertyRegistryInternal::new("profile-files"),
            env_default: PropertyRegistryInternal::new("default-files"),
        })
    }
}

#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
impl DescFromEnvironment for FileConfig {
    fn key_desc(env: &mut SalakDescContext<'_>) {
        env.add_key_desc::<Option<String>>("dir", None, None, None);
        env.add_key_desc::<String>("filename", Some(false), Some("app"), None);
        env.add_key_desc::<String>("profile", Some(false), Some("default"), None);
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
    pub(crate) fn new(
        env: &PropertyRegistryInternal<'_>,
        iorefs: &Mutex<Vec<Box<dyn IORefT + Send>>>,
    ) -> Res<Self> {
        env.require::<FileConfig>(PREFIX, iorefs)
    }

    #[allow(dead_code)]
    pub(crate) fn register_to_env(self, env: &mut PropertyRegistryInternal<'_>) {
        env.register_by_ref(Box::new(self.env_profile));
        env.register_by_ref(Box::new(self.env_default));
    }

    #[allow(dead_code)]
    pub(crate) fn build<F: Fn(FileItem) -> Res<S>, S: PropertySource + 'static>(
        &mut self,
        ext: &str,
        f: F,
    ) -> Void {
        fn make<F: Fn(FileItem) -> Res<S>, S: PropertySource + 'static>(
            f: F,
            file: String,
            dir: &Option<String>,
            env: &mut PropertyRegistryInternal<'_>,
        ) -> Void {
            let mut path = PathBuf::new();
            if let Some(d) = &dir {
                path.push(d);
            }
            path.push(file);
            if path.exists() {
                env.register_by_ref(Box::new((f)(FileItem(path))?));
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

#[derive(Debug, Clone)]
pub(crate) struct FileItem(PathBuf);

#[allow(dead_code)]
impl FileItem {
    pub(crate) fn load(&self) -> Res<String> {
        Ok(std::fs::read_to_string(self.0.clone())?)
    }

    pub(crate) fn name(&self) -> String {
        self.0.as_path().display().to_string()
    }
}

#[cfg(test)]
mod tests {
    use raw_ioref::IORef;

    use crate::{
        source::{Key, SubKeys},
        *,
    };

    struct Reload(u64);

    impl PropertySource for Reload {
        fn name(&self) -> &str {
            "reload"
        }

        fn get_property(&self, _: &Key<'_>) -> Option<Property<'_>> {
            Some(Property::I(self.0 as i64))
        }

        fn get_sub_keys<'a>(&'a self, _: &Key<'_>, _: &mut SubKeys<'a>) {}

        fn is_empty(&self) -> bool {
            false
        }
        fn reload_source(&self) -> Result<Option<Box<dyn PropertySource>>, PropertyError> {
            Ok(Some(Box::new(Reload(self.0 + 1))))
        }
    }

    #[test]
    fn reload_test() {
        let mut env = Salak::new().unwrap();
        env.register(Reload(0));
        let u8ref = env.require::<IORef<u8>>("").unwrap();
        assert_eq!(0, u8ref.get_val().unwrap());
        env.reload().unwrap();
        assert_eq!(1, u8ref.get_val().unwrap());
        env.reload().unwrap();
        assert_eq!(1, u8ref.get_val().unwrap());
    }
}
