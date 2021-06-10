use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
    sync::{Arc, Mutex, RwLock},
};

use crate::*;

#[cfg_attr(docsrs, doc(cfg(feature = "app")))]
/// Resource can be initialized in a standard way by [`Salak`].
///
/// Resource can be configured by
/// * Configuration properties by [`Resource::Config`].
/// * Customized by [`Resource::Customizer`].
/// * Other [`Resource`]s get by [`Factory`].
///
pub trait Resource: Sized {
    /// Configuration properties for current resource.
    type Config: PrefixedFromEnvironment;
    /// Customize current resource, usually by coding.
    type Customizer;

    /// Create resource, all initialization is implemented at this
    /// function. Use proper config, leave users to customizing
    /// current resource, and also request for other resources.
    fn create(
        config: Self::Config,
        factory: &FactoryContext<'_>,
        customizer: impl FnOnce(&mut Self::Customizer, &Self::Config) -> Void,
    ) -> Res<Self>;

    /// Register dependent resources. Create resource will only
    /// request for other resources, if the resource is not
    /// registered yet by [`SalakBuilder::register_resource`],
    /// an error will occure during the creating process.
    /// You may either register the resource using this function
    /// or leave the user to register.
    ///
    /// The guideline of where to register resource is to find out
    /// the boundary of resources. If you developing a service,
    /// and it depends some database resources, then you should
    /// leave the user to register database resource. If you are
    /// developing a database resource, and you need some other
    /// resources that used only by this database resource, you
    /// should treat them as a whole logical resource, and the
    /// database resource has responsibility for registering the
    /// dependent resources.
    fn register_dependent_resources(_: &mut FactoryBuilder<'_>) {}
}

#[allow(missing_debug_implementations)]
#[cfg_attr(docsrs, doc(cfg(feature = "app")))]
/// Context for implementing [`Resource`], which can get dependent
/// resources. If the dependent resource is not initialized yet,
/// it will be initialized first.
///
/// Since the api for requesting resource only getting the resource
/// wrapped by [`Arc`]. All resources can be shared, so if you want
/// the raw value, you should create by yourself, not using the
/// resource pattern.
///
pub struct FactoryContext<'a> {
    fac: &'a Salak,
    namespace: &'static str,
}

impl FactoryContext<'_> {
    /// Users can use this value to get resources in the same
    /// namespace.
    pub fn current_namespace(&self) -> &'static str {
        self.namespace
    }

    /// Get resource with default namespace. The resource will be
    /// initialized if it does not exist yet.
    pub fn get_resource<R: Resource + Send + Sync + Any>(&self) -> Res<Arc<R>> {
        self.get_resource_by_namespace("")
    }
    /// Get resource with namespace. The resource will be
    /// initialized if it does not exist yet.
    pub fn get_resource_by_namespace<R: Resource + Send + Sync + Any>(
        &self,
        namespace: &'static str,
    ) -> Res<Arc<R>> {
        #[cfg(feature = "log")]
        log::info!(
            "Request for resource ({}) at namespace [{}].",
            std::any::type_name::<R>(),
            namespace
        );
        self.fac.res.get_ref(namespace, self.fac, false)
    }

    /// Get all resouces with same type.
    pub fn get_all_resources<R: Resource + Send + Sync + Any>(&self) -> Res<Vec<Arc<R>>> {
        self.fac.res.get_all_refs(self.fac, false)
    }
}

/// Register dependent resources under same namespace.
///
/// Only relavent resources can be registered by current
/// resource. With this restriction, we can easily extend
/// resource to multiple instances.
#[cfg_attr(docsrs, doc(cfg(feature = "app")))]
#[allow(missing_debug_implementations)]
pub struct FactoryBuilder<'a> {
    builder: &'a mut ResourceRegistry,
    namespace: &'static str,
}

impl FactoryBuilder<'_> {
    /// Register dependent resource under current namespace.
    pub fn register_resource<R: Resource + Send + Sync + Any>(&mut self) {
        self.builder
            .register::<R>(ResourceBuilder::new(self.namespace));
    }

    /// Register dependent resource under current namespace
    /// with customizer.
    pub fn register_resource_with_customizer<R: Resource + Send + Sync + Any>(
        &mut self,
        customizer: impl FnOnce(&mut R::Customizer, &R::Config) -> Void + Send + Sync + 'static,
    ) {
        self.builder
            .register::<R>(ResourceBuilder::new(self.namespace).customize(customizer));
    }
}

#[cfg_attr(docsrs, doc(cfg(feature = "app")))]
/// A simple resource without config & customizer.
/// Service only care about how to get dependent resources,
/// and do not register any dependent resource.
pub trait Service: Sized {
    /// Create service by factory.
    fn create(factory: &FactoryContext<'_>) -> Res<Self>;
}

impl<T: Service> Resource for T {
    type Config = ();

    type Customizer = ();

    fn create(
        _: Self::Config,
        factory: &FactoryContext<'_>,
        _: impl FnOnce(&mut Self::Customizer, &Self::Config) -> Void,
    ) -> Res<Self> {
        T::create(factory)
    }
}

/// Factory is a resource manager. It provides a group of functions
/// to manage resource and their dependencies. Users may use
/// factory to package all components of one logic unit, such as
/// redis client configuration resource, together.
///
/// In a production
/// ready redis client configuration, we may need configuration
/// to specify redis host, port, etc, and we also need to
/// set some callbacks for monitoring the client. So we can make
/// the redis client configuration as resource, it will register
/// redis client resource, redis monitor resource, and other
/// relative resources.
///
/// In redis client resource, it needs expose configuration for
/// users to specify basic parameters for initializing redis
/// client.
///
/// In redis monitor resource, it may need other common resource
/// such as how to send metrics. So it's responsibility is
/// collecting the redis metrics and use common metric resource
/// to send the metrics.
///
/// And other resources may be added in the redis client
/// configuration.
///
/// Users may register redis client configuration resource to
/// initializing all of these resources. By using namespace,
/// users can easily create multiple group instances of same
/// type resource.
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

struct Init(Box<dyn FnOnce(&Salak, &Mutex<ResVal>) -> Void + Send>);

impl<R: Resource + Send + Sync + 'static> ResourceBuilder<R> {
    #[inline]
    fn into_init(self) -> Init {
        Init(Box::new(move |env, val| {
            let v = Box::new(Arc::new(env.init_resource_with_builder::<R>(self)?));
            let mut g = val.lock().unwrap();
            *g = v;
            Ok(())
        }))
    }
}

type ResVal = Box<dyn Any + Send + Sync>;

/// ResourceHolder is [`Sync`] and [`Send`] only when value in box is [`Send`].
struct ResourceHolder(Mutex<ResVal>, Mutex<Option<Init>>);

impl ResourceHolder {
    fn new<R: Resource + Send + Sync + 'static>(builder: ResourceBuilder<R>) -> Self {
        Self(
            Mutex::new(Box::new(0u8)),
            Mutex::new(Some(builder.into_init())),
        )
    }

    #[inline]
    fn init(&self, env: &Salak) -> Void {
        let mut guard = self.1.lock().unwrap();
        if let Some(b) = guard.take() {
            drop(guard);
            return (b.0)(env, &self.0);
        }
        Ok(())
    }

    fn get_or_init<R: Resource + Send + Sync + 'static>(
        &self,
        env: &Salak,
        _namespace: &'static str,
        query_only: bool,
    ) -> Res<Arc<R>> {
        let guard = self.0.lock().unwrap();
        if let Some(val) = guard.downcast_ref::<Arc<R>>() {
            return Ok(val.clone());
        }
        if query_only {
            return Err(PropertyError::ResourceNotFound);
        }
        #[cfg(feature = "log")]
        log::info!(
            "Init resource ({}) at namespace [{}].",
            std::any::type_name::<R>(),
            _namespace
        );
        drop(guard);
        self.init(env)?;
        self.get_or_init(env, _namespace, true)
    }
}

pub(crate) struct ResourceRegistry(BTreeMap<TypeId, BTreeMap<&'static str, ResourceHolder>>);

impl ResourceRegistry {
    pub(crate) fn new() -> Self {
        Self(BTreeMap::new())
    }

    pub(crate) fn initialize(&self, env: &Salak) -> Void {
        for x in self.0.values() {
            for r in x.values() {
                r.init(env)?;
            }
        }
        Ok(())
    }

    #[inline]
    pub(crate) fn register<R: Resource + Send + Sync + Any>(
        &mut self,
        builder: ResourceBuilder<R>,
    ) {
        let namespace = builder.namespace;
        let _ = self
            .0
            .entry(TypeId::of::<R>())
            .or_insert_with(|| BTreeMap::new())
            .entry(namespace)
            .or_insert_with(move || {
                #[cfg(feature = "log")]
                log::info!(
                    "Register resource ({}) at namespace [{}].",
                    std::any::type_name::<R>(),
                    namespace
                );
                ResourceHolder::new(builder)
            });
        R::register_dependent_resources(&mut FactoryBuilder {
            builder: self,
            namespace,
        });
    }

    #[inline]
    fn get_ref<R: Resource + Send + Sync + Any>(
        &self,
        namespace: &'static str,
        env: &Salak,
        query_only: bool,
    ) -> Res<Arc<R>> {
        if let Some(v) = self
            .0
            .get(&TypeId::of::<R>())
            .and_then(|f| f.get(namespace))
        {
            return v.get_or_init(env, namespace, query_only);
        }
        Err(PropertyError::ResourceNotFound)
    }

    fn get_all_refs<R: Resource + Send + Sync + Any>(
        &self,
        env: &Salak,
        query_only: bool,
    ) -> Res<Vec<Arc<R>>> {
        let mut r = vec![];
        for map in self.0.get(&TypeId::of::<R>()) {
            for (namespace, v) in map {
                r.push(v.get_or_init(env, namespace, query_only)?);
            }
        }
        Ok(r)
    }
}

#[cfg_attr(docsrs, doc(cfg(feature = "app")))]
impl Factory for Salak {
    #[inline]
    fn get_resource_by_namespace<R: Resource + Send + Sync + Any>(
        &self,
        namespace: &'static str,
    ) -> Result<Arc<R>, PropertyError> {
        self.res.get_ref(namespace, self, true)
    }

    #[inline]
    fn init_resource_with_builder<R: Resource>(&self, builder: ResourceBuilder<R>) -> Res<R> {
        let config = if builder.namespace.is_empty() {
            self.require::<R::Config>(<R::Config>::prefix())
        } else {
            self.require::<R::Config>(&format!("{}.{}", <R::Config>::prefix(), builder.namespace))
        }?;
        R::create(
            config,
            &FactoryContext {
                fac: self,
                namespace: builder.namespace,
            },
            builder.customizer,
        )
    }
}

#[cfg_attr(docsrs, doc(cfg(feature = "app")))]
/// Resource builder.
#[allow(missing_debug_implementations)]
pub struct ResourceBuilder<R: Resource> {
    pub(crate) namespace: &'static str,
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
