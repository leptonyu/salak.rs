use crate::*;
use std::collections::HashMap;

impl<P: FromProperty> FromEnvironment for P {
    fn from_env(
        n: &str,
        property: Option<Property>,
        _: &impl Environment,
    ) -> Result<Self, PropertyError> {
        if let Some(p) = property {
            return P::from_property(p);
        }
        P::from_err(PropertyError::NotFound(n.to_owned()))
    }
}

impl<P: FromEnvironment> FromEnvironment for Option<P> {
    fn from_env(
        n: &str,
        property: Option<Property>,
        env: &impl Environment,
    ) -> Result<Self, PropertyError> {
        match P::from_env(n, property, env) {
            Ok(a) => Ok(Some(a)),
            Err(err) => Self::from_err(err),
        }
    }
    fn from_err(err: PropertyError) -> Result<Self, PropertyError> {
        match err {
            PropertyError::NotFound(_) => Ok(None),
            _ => Err(err),
        }
    }
    fn check_is_empty(&self) -> bool {
        self.is_none()
    }

    fn load_default() -> Vec<(String, Property)> {
        P::load_default()
    }
}

impl<P: FromEnvironment> FromEnvironment for Vec<P> {
    fn from_env(
        name: &str,
        _: Option<Property>,
        env: &impl Environment,
    ) -> Result<Self, PropertyError> {
        let mut vs = vec![];
        let mut i = 0;
        let mut key = format!("{}[{}]", &name, i);
        while let Some(v) =
            <Option<P>>::from_env(&key, env.require::<Option<Property>>(&key)?, env)?
        {
            if v.check_is_empty() {
                break;
            }
            vs.push(v);
            i += 1;
            key = format!("{}[{}]", &name, i);
        }
        Ok(vs)
    }
    fn check_is_empty(&self) -> bool {
        self.is_empty()
    }
}

impl<T, S> FromEnvironment for HashSet<T, S>
where
    T: Eq + Hash + FromEnvironment,
    S: BuildHasher + Default,
{
    fn from_env(
        name: &str,
        p: Option<Property>,
        env: &impl Environment,
    ) -> Result<Self, PropertyError> {
        Ok(<Vec<T>>::from_env(name, p, env)?.into_iter().collect())
    }
    fn check_is_empty(&self) -> bool {
        self.is_empty()
    }
}

impl<T: FromEnvironment> FromEnvironment for HashMap<String, T> {
    fn from_env(
        name: &str,
        _: Option<Property>,
        env: &impl Environment,
    ) -> Result<Self, PropertyError> {
        let mut v = HashMap::new();
        for k in env.find_keys(name).into_iter() {
            if let Some(val) = env.require::<Option<T>>(&format!("{}{}", name.to_prefix(), &k))? {
                v.insert(k, val);
            }
        }
        Ok(v)
    }
    fn check_is_empty(&self) -> bool {
        self.is_empty()
    }
}
