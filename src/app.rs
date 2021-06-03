use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::{Arc, Mutex, RwLock},
};

use crate::{
    DescFromEnvironment, Environment, PrefixedFromEnvironment, PropertyError, Salak, SalakBuilder,
};

#[cfg_attr(docsrs, doc(cfg(feature = "app")))]
/// Resource can be built from [`PrefixedFromEnvironment`], and
/// also be customized by customizer.
pub trait Resource: Sized {
    /// Configuration that current resource built from.
    type Config: PrefixedFromEnvironment;
    /// Customize current resource, usually configure by coding.
    type Customizer: Default;

    /// Create resource by config and customizer.
    fn create(config: Self::Config, customizer: Self::Customizer) -> Result<Self, PropertyError>;
}

#[cfg_attr(docsrs, doc(cfg(feature = "app")))]
/// Resource builder.
#[allow(missing_debug_implementations)]
pub struct ResourceBuilder<R: Resource> {
    namespace: &'static str,
    customizer: Box<dyn FnOnce(&mut R::Customizer, &R::Config) -> Result<(), PropertyError>>,
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
        cust: impl FnOnce(&mut R::Customizer, &R::Config) -> Result<(), PropertyError> + 'static,
    ) -> Self {
        self.customizer = Box::new(cust);
        self
    }
}

#[cfg_attr(docsrs, doc(cfg(feature = "app")))]
/// Application provides standart ways for initializing [`Resource`].
pub trait Application: Environment {
    /// Initialize [`Resource`].
    fn init<R: Resource>(&self) -> Result<R, PropertyError> {
        self.init_with_builder(ResourceBuilder::default())
    }

    /// Initialize [`Resource`] with builder.
    fn init_with_builder<R: Resource>(
        &self,
        builder: ResourceBuilder<R>,
    ) -> Result<R, PropertyError>;
}

#[cfg_attr(docsrs, doc(cfg(feature = "app")))]
impl Application for Salak {
    fn init_with_builder<R: Resource>(
        &self,
        builder: ResourceBuilder<R>,
    ) -> Result<R, PropertyError> {
        let config = if builder.namespace.is_empty() {
            self.require::<R::Config>(R::Config::prefix())
        } else {
            self.require::<R::Config>(&format!("{}.{}", R::Config::prefix(), builder.namespace))
        }?;
        let mut customizer = R::Customizer::default();
        (builder.customizer)(&mut customizer, &config)?;
        R::create(config, customizer)
    }
}

impl SalakBuilder {
    /// Configure resource description.
    #[cfg_attr(docsrs, doc(cfg(feature = "app")))]
    pub fn configure_resource_description<R: Resource>(self) -> Self
    where
        R::Config: DescFromEnvironment,
    {
        self.configure_description::<R::Config>()
    }

    /// Configure resource description.
    #[cfg_attr(docsrs, doc(cfg(feature = "app")))]
    pub fn configure_resource_description_by_namespace<R: Resource>(
        self,
        namespace: &'static str,
    ) -> Self
    where
        R::Config: DescFromEnvironment,
    {
        self.configure_description_by_namespace::<R::Config>(namespace)
    }

    /// Configure resource description.
    #[cfg_attr(docsrs, doc(cfg(feature = "app")))]
    pub fn configure_resource_description_by_builder<R: Resource>(
        self,
        builder: &ResourceBuilder<R>,
    ) -> Self
    where
        R::Config: DescFromEnvironment,
    {
        self.configure_description_by_namespace::<R::Config>(builder.namespace)
    }
}

impl Resource for () {
    type Config = ();
    type Customizer = ();
    fn create(_config: Self::Config, _customizer: Self::Customizer) -> Result<Self, PropertyError> {
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
                customizer: Self::Customizer,
            ) -> Result<Self, PropertyError> {
                Ok($x::new(T::create(config, customizer)?))
            }
        }
    )+};
}

impl_container!(Cell RefCell Mutex Rc Arc RwLock);

/// Define resource.
#[macro_export]
macro_rules! define_resource {
  ($x:ident {$($f:ident:$ty:ty, $builder:expr)+}) => {
    #[allow(missing_debug_implementations, missing_copy_implementations, dead_code)]
    pub struct $x {
      $($f: $ty,)+
    }

    pub(crate) fn init() -> Result<(Salak, $x), PropertyError> {
        $(let $f = $builder;)+
        let env = Salak::builder()
            $(.configure_resource_description_by_builder::<$ty>(&$f))+
            .configure_args(app_info!())
            .build()?;
        $(let $f = env.init_with_builder($f)?;)+
        let val = $x{$($f,)+};
        Ok((env, val))
    }

  }
}
