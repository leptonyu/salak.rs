use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
    sync::{Arc, Mutex, RwLock},
    time::Duration,
};

use crate::{Environment, PrefixedFromEnvironment, PropertyError, Res, Salak, SalakBuilder, Void};

#[cfg_attr(docsrs, doc(cfg(feature = "app")))]
/// Resource can be initialized in a standard way by [`Salak`].
///
/// Resource can config by
/// * Configuration properties by [`Resource::Config`].
/// * Customized by [`Resource::Customizer`].
/// * Other [`Resource`]s get by [`Factory`].
///
pub trait Resource: Sized {
    /// Configuration properties for current resource.
    type Config: PrefixedFromEnvironment;
    /// Customize current resource, usually configure by coding.
    type Customizer;

    /// Create resource.
    fn create(
        config: Self::Config,
        factory: &FactoryContext<'_>,
        customizer: impl FnOnce(&mut Self::Customizer, &Self::Config) -> Void,
    ) -> Res<Self>;
}

/// Get dependent resource when creating resource.
#[allow(missing_debug_implementations)]
pub struct FactoryContext<'a> {
    fac: &'a Salak,
}

impl FactoryContext<'_> {
    /// Get resource with default namespace.
    pub fn get_resource<R: Resource + Send + Sync + Any>(&self) -> Res<Arc<R>> {
        #[cfg(feature = "log")]
        log::info!("Request for resource({})", std::any::type_name::<R>());
        self.fac.get_resource()
    }
    /// Get resource with default namespace.
    pub fn get_resource_by_namespace<R: Resource + Send + Sync + Any>(
        &self,
        namespace: &'static str,
    ) -> Res<Arc<R>> {
        #[cfg(feature = "log")]
        log::info!(
            "Request for resource({}) at namespace {}",
            std::any::type_name::<R>(),
            namespace
        );
        self.fac.get_resource_by_namespace(namespace)
    }
}

/// Factory is a resource manager for initializing resource or getting resource from cache.
#[cfg_attr(docsrs, doc(cfg(feature = "app")))]
pub trait Factory: Environment {
    #[inline]
    /// Get resource [`Arc<R>`] from cache with default namespace. Users can customize
    /// the resource by [`SalakBuilder::register_resource()`].
    fn get_resource<R: Resource + Send + Sync + Any>(&self) -> Res<Arc<R>> {
        self.get_resource_by_namespace("")
    }
    /// Get resource [`Arc<R>`] from cache by namespace. Users can customize
    /// the resource by [`SalakBuilder::register_resource()`].
    fn get_resource_by_namespace<R: Resource + Send + Sync + Any>(
        &self,
        namespace: &'static str,
    ) -> Res<Arc<R>>;

    /// Initialize [`Resource`].
    fn init_resource<R: Resource>(&self) -> Res<R> {
        self.init_resource_with_builder(ResourceBuilder::default())
    }

    /// Initialize [`Resource`] with builder.
    fn init_resource_with_builder<R: Resource>(&self, builder: ResourceBuilder<R>) -> Res<R>;
}

impl Resource for () {
    type Config = ();
    type Customizer = ();

    fn create(
        _: Self::Config,
        _: &FactoryContext<'_>,
        _: impl FnOnce(&mut Self::Customizer, &Self::Config) -> Void,
    ) -> Res<Self> {
        Ok(())
    }
}

macro_rules! impl_container {
    ($($x:ident)+) => {$(
        impl<T: Resource> Resource for $x<T> {
            type Config = T::Config;
            type Customizer = T::Customizer;

            fn create(
                config: Self::Config,
                factory: &FactoryContext<'_>,
                customizer: impl FnOnce(&mut Self::Customizer, &Self::Config) -> Result<(), PropertyError>,
            ) -> Result<Self, PropertyError> {
                Ok($x::new(T::create(config, factory, customizer)?))
            }
        }
    )+};
}

impl_container!(Cell RefCell Mutex Rc Arc RwLock);

struct Init(Box<dyn FnOnce(&Salak) -> Res<Box<dyn Any>>>);

impl<R: Resource + 'static> ResourceBuilder<R> {
    fn into_init(self) -> Init {
        Init(Box::new(move |env| {
            env.init_resource_with_builder(self).map(|v| {
                let v: Box<dyn Any> = Box::new(Arc::new(v));
                v
            })
        }))
    }
}

type ResVal = (Box<dyn Any>, Option<Init>);

/// ResourceHolder is [`Sync`] and [`Send`] only when value in box is [`Send`].
struct ResourceHolder(Mutex<ResVal>, Box<dyn Fn(&Salak, &ResourceHolder) -> Void>);

impl ResourceHolder {
    fn new<R: Resource + Send + 'static>(builder: ResourceBuilder<R>) -> Self {
        let namespace = builder.namespace.clone();
        Self(
            Mutex::new((Box::new(0u8), Some(builder.into_init()))),
            Box::new(move |env, holder| holder.get::<R>(env, namespace).map(|_| ())),
        )
    }

    #[inline]
    fn init(&self, env: &Salak) -> Void {
        (self.1)(env, self)
    }

    fn get<R: Resource + Send + 'static>(
        &self,
        env: &Salak,
        _namespace: &'static str,
    ) -> Res<Arc<R>> {
        loop {
            let mut guard = self.0.lock().unwrap();
            if let Some(val) = guard.0.downcast_ref::<Arc<R>>() {
                return Ok(val.clone());
            }
            #[cfg(feature = "log")]
            log::info!(
                "Init resource ({}) at namespace {}",
                std::any::type_name::<R>(),
                _namespace
            );
            if let Some(i) = guard.0.downcast_mut::<u8>() {
                if *i == 1 {
                    drop(guard);
                    std::thread::sleep(Duration::from_millis(1));
                    continue;
                } else if *i != 0 {
                    return Err(PropertyError::ResourceNotFound);
                }
                *i = 1;
            } else {
                return Err(PropertyError::ResourceNotFound);
            }
            let ret = match guard.1.take() {
                Some(init) => init,
                _ => {
                    guard.0 = Box::new(2u8);
                    return Err(PropertyError::ResourceNotFound);
                }
            };
            drop(guard);
            let ret = (ret.0)(env);
            let mut guard = self.0.lock().unwrap();
            return ret
                .and_then(|op| {
                    op.downcast::<Arc<R>>()
                        .map(|v| {
                            guard.0 = v.clone();
                            *v
                        })
                        .map_err(|_| PropertyError::ResourceNotFound)
                })
                .map_err(|e| {
                    guard.0 = Box::new(3u8);
                    e
                });
        }
    }
}

pub(crate) struct ResourceRegistry(HashMap<TypeId, HashMap<&'static str, ResourceHolder>>);

impl ResourceRegistry {
    pub(crate) fn new() -> Self {
        Self(HashMap::new())
    }

    pub(crate) fn initialize(&self, env: &Salak) -> Void {
        for x in self.0.values() {
            for r in x.values() {
                r.init(env)?;
            }
        }
        Ok(())
    }

    fn register<R: Resource + Send + Sync + Any>(&mut self, builder: ResourceBuilder<R>) {
        let _ = self
            .0
            .entry(TypeId::of::<R>())
            .or_insert_with(|| HashMap::new())
            .entry(builder.namespace)
            .or_insert_with(move || ResourceHolder::new(builder));
    }

    #[inline]
    fn get_ref<R: Resource + Send + Sync + Any>(
        &self,
        namespace: &'static str,
        env: &Salak,
    ) -> Res<Arc<R>> {
        if let Some(v) = self
            .0
            .get(&TypeId::of::<R>())
            .and_then(|f| f.get(namespace))
        {
            return v.get(env, namespace);
        }
        Err(PropertyError::ResourceNotFound)
    }
}

#[cfg_attr(docsrs, doc(cfg(feature = "app")))]
impl SalakBuilder {
    #[inline]
    #[cfg_attr(docsrs, doc(cfg(feature = "app")))]
    /// Register [`Resource`] with default builder.
    pub fn register_default_resource<R: Resource + Send + Sync + Any>(self) -> Self {
        self.register_resource::<R>(ResourceBuilder::default())
    }

    #[inline]
    #[cfg_attr(docsrs, doc(cfg(feature = "app")))]
    /// Register [`Resource`] by [`ResourceBuilder`].
    pub fn register_resource<R: Resource + Send + Sync + Any>(
        self,
        builder: ResourceBuilder<R>,
    ) -> Self {
        let mut env = self.configure_resource_description_by_builder(&builder);
        env.resource.register(builder);
        env
    }

    #[inline]
    /// Configure resource description.
    #[cfg_attr(docsrs, doc(cfg(feature = "app")))]
    pub(crate) fn configure_resource_description_by_builder<R: Resource>(
        self,
        builder: &ResourceBuilder<R>,
    ) -> Self {
        self.configure_description_by_namespace::<R::Config>(builder.namespace)
    }
}

#[cfg_attr(docsrs, doc(cfg(feature = "app")))]
impl Factory for Salak {
    #[inline]
    fn get_resource_by_namespace<R: Resource + Send + Sync + Any>(
        &self,
        namespace: &'static str,
    ) -> Result<Arc<R>, PropertyError> {
        self.res.get_ref(namespace, self)
    }

    fn init_resource_with_builder<R: Resource>(&self, builder: ResourceBuilder<R>) -> Res<R> {
        let config = if builder.namespace.is_empty() {
            self.require::<R::Config>(<R::Config>::prefix())
        } else {
            self.require::<R::Config>(&format!("{}.{}", <R::Config>::prefix(), builder.namespace))
        }?;
        R::create(config, &FactoryContext { fac: self }, builder.customizer)
    }
}

#[cfg_attr(docsrs, doc(cfg(feature = "app")))]
/// Resource builder.
#[allow(missing_debug_implementations)]
pub struct ResourceBuilder<R: Resource> {
    namespace: &'static str,
    customizer: Box<dyn FnOnce(&mut R::Customizer, &R::Config) -> Void + Send>,
}

impl<R: Resource> Default for ResourceBuilder<R> {
    fn default() -> Self {
        Self {
            namespace: "",
            customizer: Box::new(|_, _| Ok(())),
        }
    }
}

impl<R: Resource> ResourceBuilder<R> {
    /// Create resource builder by namespace.
    pub fn new(namespace: &'static str) -> Self {
        ResourceBuilder::default().namespace(namespace)
    }

    /// Configure namespace.
    pub fn namespace(mut self, namespace: &'static str) -> Self {
        self.namespace = namespace;
        self
    }

    /// Configure customize.
    pub fn customize(
        mut self,
        cust: impl FnOnce(&mut R::Customizer, &R::Config) -> Void + Send + Sync + 'static,
    ) -> Self {
        self.customizer = Box::new(cust);
        self
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    #[test]
    fn app_test() {
        let env = Salak::builder().build().unwrap();
        let v = env.get_resource::<()>();
        assert_eq!(true, v.is_err());
        let env = Salak::builder()
            .register_default_resource::<()>()
            .build()
            .unwrap();
        let v = env.get_resource::<()>();
        assert_eq!(true, v.is_ok());
    }
}
