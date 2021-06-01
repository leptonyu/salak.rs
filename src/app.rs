use std::marker::PhantomData;

use crate::{Environment, PrefixedFromEnvironment, PropertyError, Salak};

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
/// Application provides standart ways for initializing [`Resource`].
pub trait Application: Environment {
    /// Initialize [`Resource`].
    fn init<R: Resource>(&self) -> Result<R, PropertyError> {
        self.init_with_namespace("")
    }

    /// Initialize [`Resource`] with customizer.
    fn init_with_customizer<R: Resource>(
        &self,
        customize: impl Fn(&mut R::Customizer, &R::Config) -> Result<(), PropertyError>,
    ) -> Result<R, PropertyError> {
        self.init_by_namespace_and_customizer("", customize)
    }

    /// Initialize [`Resource`] with customizer.
    fn init_with_namespace<R: Resource>(&self, namespace: &str) -> Result<R, PropertyError> {
        self.init_by_namespace_and_customizer(namespace, |_, _| Ok(()))
    }

    /// Initialize [`Resource`] with namespace and customizer.
    fn init_by_namespace_and_customizer<R: Resource>(
        &self,
        namespace: &str,
        customize: impl Fn(&mut R::Customizer, &R::Config) -> Result<(), PropertyError>,
    ) -> Result<R, PropertyError>;
}

#[cfg_attr(docsrs, doc(cfg(feature = "app")))]
impl Application for Salak {
    fn init_by_namespace_and_customizer<R: Resource>(
        &self,
        namespace: &str,
        customize: impl Fn(&mut R::Customizer, &R::Config) -> Result<(), PropertyError>,
    ) -> Result<R, PropertyError> {
        let config = if namespace.is_empty() {
            self.require::<R::Config>(R::Config::prefix())
        } else {
            self.require::<R::Config>(&format!("{}.{}", R::Config::prefix(), namespace))
        }?;
        let mut customizer = R::Customizer::default();
        (customize)(&mut customizer, &config)?;
        R::create(config, customizer)
    }
}

#[cfg_attr(docsrs, doc(cfg(feature = "app")))]
/// Lazy resource.
#[allow(missing_debug_implementations)]
pub struct Lazy<T: Resource> {
    config: T::Config,
    customizer: T::Customizer,
    _data: PhantomData<T>,
}

impl<R: Resource> Resource for Lazy<R> {
    type Config = R::Config;
    type Customizer = R::Customizer;

    fn create(config: Self::Config, customizer: Self::Customizer) -> Result<Self, PropertyError> {
        Ok(Lazy {
            config,
            customizer,
            _data: PhantomData,
        })
    }
}

impl<T: Resource> Lazy<T> {
    /// Initialize lazy resource.
    pub fn finish(self) -> Result<T, PropertyError> {
        T::create(self.config, self.customizer)
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[derive(Debug, FromEnvironment)]
    #[salak(prefix = "config")]
    struct Conf {
        name: String,
    }

    struct A;

    impl Resource for A {
        type Config = Conf;

        type Customizer = ();

        fn create(c: Self::Config, _: Self::Customizer) -> Result<Self, PropertyError> {
            println!("hello {}", c.name);
            Ok(A)
        }
    }

    struct B {
        lazy: Lazy<A>,
        def: A,
    }

    impl B {
        fn create(env: &impl Application) -> Result<Self, PropertyError> {
            Ok(B {
                lazy: env.init_with_namespace("lazy")?,
                def: env.init()?,
            })
        }
    }

    #[test]
    fn resource_test() {
        let env = Salak::builder()
            .set("config.name", "First")
            .set("config.lazy.name", "Second")
            .build()
            .unwrap();
        let a = env.init_with_namespace::<Lazy<A>>("lazy").unwrap();
        let _ = env.init::<A>().unwrap();
        let _ = a.finish().unwrap();

        let c = B::create(&env).unwrap();
        let _ = c.lazy.finish().unwrap();
        let _ = c.def;
    }
}
