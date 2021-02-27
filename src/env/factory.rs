use crate::*;
use std::any::Any;
use std::any::TypeId;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::Mutex;

/// Factory smart pointer reference.
#[derive(Debug)]
pub struct FacRef<T: Sized> {
    value: Arc<dyn Any + Send + Sync>,
    _data: PhantomData<T>,
}

impl<T: 'static> Deref for FacRef<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        (*self.value).downcast_ref().expect(NOT_POSSIBLE)
    }
}

/// Factory builder scope.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum FactoryScope {
    /// Singleton.
    Singleton,
    /// Always create a new one.
    AlwaysNewCreated,
}

/// A factory.
pub trait Factory: Sized {
    /// Factory environment.
    type Env: Environment;

    /// Get environment.
    fn env(&self) -> &Self::Env;

    /// Get reference of specified type.
    /// If [`FactoryScope`] is [`FactoryScope::Singleton`], then return cached value,
    /// otherwise create a new one.
    fn get_or_build<T: FromFactory>(&self) -> Result<FacRef<T>, PropertyError>;

    /// Initialize some value.
    fn init<T: FromFactory>(&self) -> Result<(), PropertyError> {
        let _ = self.get_or_build::<T>()?;
        Ok(())
    }
}

pub(crate) struct FactoryRegistry<T: Environment> {
    pub(crate) env: T,
    repository: Mutex<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
}

impl<E: Environment> FactoryRegistry<E> {
    pub(crate) fn new(env: E) -> Self {
        Self {
            env,
            repository: Mutex::new(HashMap::new()),
        }
    }
}

impl<E: Environment> Factory for FactoryRegistry<E> {
    type Env = E;
    fn env(&self) -> &Self::Env {
        &self.env
    }
    fn get_or_build<T: 'static + FromFactory>(&self) -> Result<FacRef<T>, PropertyError> {
        if T::scope() == FactoryScope::AlwaysNewCreated {
            return Ok(FacRef {
                value: Arc::new(T::build(self)),
                _data: PhantomData,
            });
        }
        let mut map = self.repository.lock().expect(NOT_POSSIBLE);
        let tid = TypeId::of::<T>();
        if map.get(&tid).is_none() {
            map.insert(tid, Arc::new(T::build(self)?));
        }
        let value = map.get(&tid).expect(NOT_POSSIBLE);
        Ok(FacRef {
            value: value.clone(),
            _data: PhantomData,
        })
    }
}

/// Build object from [`Factory`]
pub trait FromFactory: Sync + Send + Sized + Any {
    /// Actual building blocks.
    fn build(fac: &impl Factory) -> Result<Self, PropertyError>;

    /// Build scope.
    fn scope() -> FactoryScope {
        FactoryScope::Singleton
    }
}

impl<E: Environment> Environment for FactoryRegistry<E> {
    fn require<T: FromEnvironment>(&self, name: &str) -> Result<T, PropertyError> {
        self.env.require(name)
    }
    fn resolve_placeholder(&self, value: String) -> Result<Option<Property>, PropertyError> {
        self.env.resolve_placeholder(value)
    }
    fn find_keys(&self, prefix: &str) -> Vec<String> {
        self.env.find_keys(prefix)
    }
    fn reload(&mut self) -> Result<(), PropertyError> {
        self.env.reload()
    }
}

#[cfg(feature = "enable_derive")]
impl<F: 'static + Sync + Send + DefaultSourceFromEnvironment> FromFactory for F {
    fn build(fac: &impl Factory) -> Result<Self, PropertyError> {
        fac.env().load_config()
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    use rand::random;

    #[derive(Eq, PartialEq)]
    struct Connection {
        value: u64,
    }

    impl FromFactory for Connection {
        fn build(_: &impl Factory) -> Result<Self, PropertyError> {
            Ok(Connection { value: random() })
        }
    }

    #[test]
    fn cache_test() {
        let fr = FactoryRegistry::new(SourceRegistry::new());
        let a: &Connection = &*fr.get_or_build::<Connection>().unwrap();
        let b: &Connection = &*fr.get_or_build::<Connection>().unwrap();
        let c = Connection::build(&fr).unwrap();
        assert_eq!(a.value, b.value);
        assert_ne!(c.value, b.value);
    }
}
