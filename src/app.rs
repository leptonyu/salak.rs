use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
    sync::{Arc, Mutex, RwLock},
};

use crate::{Environment, PrefixedFromEnvironment, PropertyError, Res, Salak, SalakBuilder, Void};

/// Resource customizer.
#[cfg_attr(docsrs, doc(cfg(feature = "app")))]
pub trait ResourceCustomizer {
    /// Config.
    type Config: PrefixedFromEnvironment;

    /// Create resource customizer.
    fn new(fac: &impl ResourceFactory) -> Self;
}

#[cfg_attr(docsrs, doc(cfg(feature = "app")))]
/// Resource can be built from [`PrefixedFromEnvironment`], and
/// also be customized by customizer.
pub trait Resource: Sized {
    /// Customize current resource, usually configure by coding.
    type Customizer: ResourceCustomizer;

    /// Create resource by config and customizer.
    fn create(
        config: <Self::Customizer as ResourceCustomizer>::Config,
        customizer: Self::Customizer,
    ) -> Res<Self>;
}

/// Resource factory.
#[cfg_attr(docsrs, doc(cfg(feature = "app")))]
pub trait ResourceFactory: Environment {
    /// Get [`Arc<R>`].
    fn get_resource<R: Resource + Any>(&self) -> Res<Arc<R>> {
        self.get_resource_by_namespace("")
    }
    /// Get [`Arc<R>`].
    fn get_resource_by_namespace<R: Resource + Any>(&self, namespace: &'static str) -> Res<Arc<R>>;

    /// Initialize [`Resource`].
    fn init_resource<R: Resource>(&self) -> Res<R> {
        self.init_resource_with_builder(ResourceBuilder::default())
    }

    /// Initialize [`Resource`] with builder.
    fn init_resource_with_builder<R: Resource>(&self, builder: ResourceBuilder<R>) -> Res<R>;
}

impl ResourceCustomizer for () {
    type Config = ();

    fn new(_: &impl ResourceFactory) -> Self {}
}

impl Resource for () {
    type Customizer = ();
    fn create(
        _config: <Self::Customizer as ResourceCustomizer>::Config,
        _customizer: Self::Customizer,
    ) -> Res<Self> {
        Ok(())
    }
}

macro_rules! impl_container {
    ($($x:ident)+) => {$(
        impl<T: Resource> Resource for $x<T> {
            type Customizer = T::Customizer;
            fn create(
                config: <Self::Customizer as ResourceCustomizer>::Config,
                customizer: Self::Customizer,
            ) -> Result<Self, PropertyError> {
                Ok($x::new(T::create(config, customizer)?))
            }
        }
    )+};
}

impl_container!(Cell RefCell Mutex Rc Arc RwLock);

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

    fn get<R: Resource + 'static>(
        &self,
        env: &impl ResourceFactory,
        namespace: &'static str,
    ) -> Res<Arc<R>> {
        loop {
            let mut guard = self.val.lock().unwrap();
            if let Some(val) = guard.downcast_ref::<Arc<R>>() {
                return Ok(val.clone());
            }
            if let Some(()) = guard.downcast_ref::<()>() {
                continue;
            }
            let builder = match guard.downcast_mut::<ResourceBuilder<R>>() {
                Some(val) => {
                    let b = std::mem::replace(val, ResourceBuilder::default().namespace(namespace));
                    *guard = Box::new(());
                    b
                }
                _ => ResourceBuilder::default().namespace(namespace),
            };
            drop(guard);
            let v = Arc::new(env.init_resource_with_builder(builder)?);
            let mut guard = self.val.lock().unwrap();
            let _ = std::mem::replace(&mut *guard, Box::new(v.clone()));
            return Ok(v);
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

    fn get_ref<R: Resource + Any>(
        &self,
        namespace: &'static str,
        env: &impl ResourceFactory,
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
        self.configure_description_by_namespace::<<R::Customizer as ResourceCustomizer>::Config>(
            builder.namespace,
        )
    }
}

#[cfg_attr(docsrs, doc(cfg(feature = "app")))]
impl ResourceFactory for Salak {
    /// Get [`Arc<R>`] by namespace.
    fn get_resource_by_namespace<R: Resource + Any>(
        &self,
        namespace: &'static str,
    ) -> Result<Arc<R>, PropertyError> {
        self.res.get_ref(namespace, self)
    }

    /// Initialize [`Resource`] with builder.
    fn init_resource_with_builder<R: Resource>(&self, builder: ResourceBuilder<R>) -> Res<R> {
        let config = if builder.namespace.is_empty() {
            self.require::<<R::Customizer as ResourceCustomizer>::Config>(
                <<R::Customizer as ResourceCustomizer>::Config>::prefix(),
            )
        } else {
            self.require::<<R::Customizer as ResourceCustomizer>::Config>(&format!(
                "{}.{}",
                <<R::Customizer as ResourceCustomizer>::Config>::prefix(),
                builder.namespace
            ))
        }?;
        let mut customizer = R::Customizer::new(self);
        (builder.customizer)(&mut customizer, &config)?;
        R::create(config, customizer)
    }
}

#[cfg_attr(docsrs, doc(cfg(feature = "app")))]
/// Resource builder.
#[allow(missing_debug_implementations)]
pub struct ResourceBuilder<R: Resource> {
    namespace: &'static str,
    customizer:
        Box<dyn FnOnce(&mut R::Customizer, &<R::Customizer as ResourceCustomizer>::Config) -> Void>,
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
        cust: impl FnOnce(&mut R::Customizer, &<R::Customizer as ResourceCustomizer>::Config) -> Void
            + 'static,
    ) -> Self {
        self.customizer = Box::new(cust);
        self
    }
}
