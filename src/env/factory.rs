use crate::*;
use std::any::TypeId;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::Mutex;
use std::{any::Any, sync::MutexGuard};

type ASS = dyn Any + Send + Sync;

/// Factory smart pointer reference.
#[derive(Debug)]
pub struct FacRef<T: Sized> {
    value: Arc<ASS>,
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

type FacRepo = HashMap<TypeId, Arc<ASS>>;

/// Factory Context.
#[derive(Debug)]
pub struct FactoryContext<'a> {
    guard: MutexGuard<'a, FacRepo>,
}

impl FactoryContext<'_> {
    /// Get instance from factory.
    pub fn get<T: 'static + FromFactory>(
        &mut self,
        env: &impl Environment,
    ) -> Result<FacRef<T>, PropertyError> {
        if T::scope() == FactoryScope::AlwaysNewCreated {
            return Ok(FacRef {
                value: Arc::new(T::build(self, env)?),
                _data: PhantomData,
            });
        }
        let tid = TypeId::of::<T>();
        if let Some(value) = self.guard.get(&tid) {
            return Ok(FacRef {
                value: value.clone(),
                _data: PhantomData,
            });
        }
        let value = Arc::new(T::build(self, env)?);
        self.guard.insert(tid, value.clone());
        Ok(FacRef {
            value,
            _data: PhantomData,
        })
    }
}

/// Build object from [`Factory`]
pub trait FromFactory: Sync + Send + Sized + Any {
    /// Actual building blocks.
    fn build(_: &mut FactoryContext<'_>, _: &impl Environment) -> Result<Self, PropertyError>;

    /// Build scope.
    fn scope() -> FactoryScope {
        FactoryScope::Singleton
    }
}

pub(crate) struct FactoryRegistry<T: Environment> {
    pub(crate) env: T,
    repository: Mutex<FacRepo>,
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
        FactoryContext {
            guard: self.repository.lock().expect(NOT_POSSIBLE),
        }
        .get(&self.env)
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
    fn build(_: &mut FactoryContext<'_>, env: &impl Environment) -> Result<Self, PropertyError> {
        env.load_config()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::random;

    #[derive(Eq, PartialEq)]
    struct Connection {
        value: u64,
    }

    impl Connection {
        fn value(&self) -> u64 {
            self.value
        }
    }

    impl FromFactory for Connection {
        fn build(_: &mut FactoryContext<'_>, _: &impl Environment) -> Result<Self, PropertyError> {
            Ok(Connection { value: random() })
        }
    }

    struct Repository {
        conn: FacRef<Connection>,
    }

    impl FromFactory for Repository {
        fn build(
            context: &mut FactoryContext<'_>,
            env: &impl Environment,
        ) -> Result<Self, PropertyError> {
            Ok(Repository {
                conn: context.get(env)?,
            })
        }
    }
    impl Repository {
        fn value(&self) -> u64 {
            (*self.conn).value()
        }
    }
    struct PrototypeConnection {
        value: u64,
    }

    impl PrototypeConnection {
        fn value(&self) -> u64 {
            self.value
        }
    }

    impl FromFactory for PrototypeConnection {
        fn build(_: &mut FactoryContext<'_>, _: &impl Environment) -> Result<Self, PropertyError> {
            Ok(PrototypeConnection { value: random() })
        }

        fn scope() -> FactoryScope {
            FactoryScope::AlwaysNewCreated
        }
    }

    #[test]
    fn singleton_test() {
        let fr = FactoryRegistry::new(SourceRegistry::new());
        let a = fr.get_or_build::<Connection>().unwrap();
        let b = fr.get_or_build::<Connection>().unwrap();
        assert_eq!(a.value(), b.value());
        let r = fr.get_or_build::<Repository>().unwrap();
        assert_eq!(a.value(), r.value());
    }

    #[test]
    fn prototype_test() {
        let fr = FactoryRegistry::new(SourceRegistry::new());
        let a = fr.get_or_build::<PrototypeConnection>().unwrap();
        let b = fr.get_or_build::<PrototypeConnection>().unwrap();
        assert_ne!(a.value(), b.value());
    }
}
