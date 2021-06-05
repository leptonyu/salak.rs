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
        factory: &impl Factory,
        customizer: impl FnOnce(&mut Self::Customizer, &Self::Config) -> Void,
    ) -> Res<Self>;
}

/// Factory is a resource manager for initializing resource or getting resource from cache.
#[cfg_attr(docsrs, doc(cfg(feature = "app")))]
pub trait Factory: Environment {
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
        _: &impl Factory,
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
                factory: &impl Factory,
                customizer: impl FnOnce(&mut Self::Customizer, &Self::Config) -> Result<(), PropertyError>,
            ) -> Result<Self, PropertyError> {
                Ok($x::new(T::create(config, factory, customizer)?))
            }
        }
    )+};
}

impl_container!(Cell RefCell Mutex Rc Arc RwLock);

/// ResourceHolder is [`Sync`] or [`Send`] only when value in box is [`Sync`] or [`Send`].
struct ResourceHolder {
    val: Mutex<Box<dyn Any>>,
}

unsafe impl Send for ResourceHolder {}
unsafe impl Sync for ResourceHolder {}

impl ResourceHolder {
    fn new<R: Resource + Send + Sync + 'static>(builder: ResourceBuilder<R>) -> Self {
        Self {
            val: Mutex::new(Box::new(builder)),
        }
    }

    fn get<R: Resource + Send + Sync + 'static>(
        &self,
        env: &impl Factory,
        namespace: &'static str,
    ) -> Res<Arc<R>> {
        let mut flag = false;
        loop {
            if flag {
                std::thread::sleep(Duration::from_millis(1));
            }
            let mut guard = self.val.lock().unwrap();
            if let Some(val) = guard.downcast_ref::<Arc<R>>() {
                return Ok(val.clone());
            }
            if let Some(i) = guard.downcast_ref::<u8>() {
                if *i > 0 {
                    return Err(PropertyError::ResourceNotFound);
                }
                flag = true;
                continue;
            }
            let builder = match guard.downcast_mut::<ResourceBuilder<R>>() {
                Some(val) => {
                    let b = std::mem::replace(val, ResourceBuilder::default().namespace(namespace));
                    *guard = Box::new(0u8);
                    b
                }
                _ => ResourceBuilder::default().namespace(namespace),
            };
            drop(guard);
            let ret = env.init_resource_with_builder(builder);
            let mut guard = self.val.lock().unwrap();
            match ret {
                Ok(builder) => {
                    let v = Arc::new(builder);
                    *guard = Box::new(v.clone());
                    return Ok(v);
                }
                Err(p) => {
                    if let Some(_) = guard.downcast_mut::<u8>() {
                        *guard = Box::new(1u8);
                    }
                    return Err(p);
                }
            }
        }
    }
}

pub(crate) struct ResourceRegistry(HashMap<TypeId, HashMap<&'static str, ResourceHolder>>);

impl ResourceRegistry {
    pub(crate) fn new() -> Self {
        Self(HashMap::new())
    }

    fn register<R: Resource + Send + Sync + Any>(&mut self, builder: ResourceBuilder<R>) {
        let _ = self
            .0
            .entry(TypeId::of::<R>())
            .or_insert_with(|| HashMap::new())
            .entry(builder.namespace)
            .or_insert_with(move || ResourceHolder::new(builder));
    }

    fn get_ref<R: Resource + Send + Sync + Any>(
        &self,
        namespace: &'static str,
        env: &impl Factory,
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

    /// Configure resource description.
    // #[cfg_attr(docsrs, doc(cfg(feature = "app")))]
    // pub(crate) fn configure_resource_description<R: Resource>(self) -> Self {
    //     self.configure_description::<R::Config>()
    // }

    // /// Configure resource description.
    // #[cfg_attr(docsrs, doc(cfg(feature = "app")))]
    // pub(crate) fn configure_resource_description_by_namespace<R: Resource>(
    //     self,
    //     namespace: &'static str,
    // ) -> Self {
    //     self.configure_description_by_namespace::<R::Config>(namespace)
    // }

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
        R::create(config, self, builder.customizer)
    }
}

#[cfg_attr(docsrs, doc(cfg(feature = "app")))]
/// Resource builder.
#[allow(missing_debug_implementations)]
pub struct ResourceBuilder<R: Resource> {
    namespace: &'static str,
    customizer: Box<dyn FnOnce(&mut R::Customizer, &R::Config) -> Void>,
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
    /// Configure namespace.
    pub fn namespace(mut self, namespace: &'static str) -> Self {
        self.namespace = namespace;
        self
    }

    /// Configure customize.
    pub fn customize(
        mut self,
        cust: impl FnOnce(&mut R::Customizer, &R::Config) -> Void + 'static,
    ) -> Self {
        self.customizer = Box::new(cust);
        self
    }
}
