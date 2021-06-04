use std::sync::{Arc, Mutex};

use crate::{
    source_raw::PropertyRegistryInternal, FromEnvironment, Property, PropertyError, Res,
    SalakContext, Void,
};

#[cfg(feature = "derive")]
use crate::{DescFromEnvironment, SalakDescContext};
/// A wrapper of `T` that can be updated when reloading configurations.
#[allow(missing_debug_implementations)]
#[derive(Clone)]
pub struct IORef<T>(pub(crate) Arc<Mutex<T>>, pub(crate) String);

pub(crate) trait IORefT: Send {
    fn reload_ref(
        &self,
        env: &PropertyRegistryInternal<'_>,
        ioref: &Mutex<Vec<Box<dyn IORefT + Send>>>,
    ) -> Void;
}

impl<T: Send + Clone + FromEnvironment> IORefT for IORef<T> {
    #[inline]
    fn reload_ref(
        &self,
        env: &PropertyRegistryInternal<'_>,
        ioref: &Mutex<Vec<Box<dyn IORefT + Send>>>,
    ) -> Void {
        self.set(env.require::<T>(&self.1, ioref)?)
    }
}

impl<T: Clone> IORef<T> {
    #[inline]
    fn new(key: &str, val: T) -> Self {
        Self(Arc::new(Mutex::new(val)), key.to_string())
    }

    #[inline]
    fn set(&self, val: T) -> Void {
        let mut guard = self
            .0
            .lock()
            .map_err(|_| PropertyError::parse_fail("IORef get fail"))?;
        *guard = val;
        Ok(())
    }

    /// Get value from reference.
    pub fn get_val(&self) -> Res<T> {
        let guard = self
            .0
            .lock()
            .map_err(|_| PropertyError::parse_fail("IORef get fail"))?;
        Ok(T::clone(&*guard))
    }
}

impl<T> FromEnvironment for IORef<T>
where
    T: Clone + FromEnvironment + Send + 'static,
{
    fn from_env(val: Option<Property<'_>>, env: &mut SalakContext<'_>) -> Res<Self> {
        let t = T::from_env(val, env)?;
        let v = IORef::new(env.current_key(), t);
        env.register_ioref(&v);
        Ok(v)
    }
}

#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
impl<T> DescFromEnvironment for IORef<T>
where
    T: Clone + DescFromEnvironment + Send + 'static,
{
    fn key_desc(env: &mut SalakDescContext<'_>) {
        T::key_desc(env);
    }
}
