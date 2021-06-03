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
    fn init_with_customizer<R: Resource, F>(&self, customize: F) -> Result<R, PropertyError>
    where
        F: Fn(&mut R::Customizer, &R::Config) -> Result<(), PropertyError>,
    {
        self.init_by_namespace_and_customizer("", customize)
    }

    /// Initialize [`Resource`] with customizer.
    fn init_with_namespace<R: Resource>(&self, namespace: &str) -> Result<R, PropertyError> {
        self.init_by_namespace_and_customizer(namespace, |_, _| Ok(()))
    }

    /// Initialize [`Resource`] with namespace and customizer.
    fn init_by_namespace_and_customizer<R: Resource, F>(
        &self,
        namespace: &str,
        customize: F,
    ) -> Result<R, PropertyError>
    where
        F: Fn(&mut R::Customizer, &R::Config) -> Result<(), PropertyError>;
}

#[cfg_attr(docsrs, doc(cfg(feature = "app")))]
impl Application for Salak {
    fn init_by_namespace_and_customizer<R: Resource, F>(
        &self,
        namespace: &str,
        customize: F,
    ) -> Result<R, PropertyError>
    where
        F: Fn(&mut R::Customizer, &R::Config) -> Result<(), PropertyError>,
    {
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

pub struct ResourceCustomizer<R: Resource> {
    namespace: &'static str,
    customizer: Box<dyn Fn(&mut R::Customizer, &R::Config) -> Result<(), PropertyError>>,
}

impl<R: Resource> Default for ResourceCustomizer<R> {
    fn default() -> Self {
        Self {
            namespace: "",
            customizer: Box::new(|_, _| Ok(())),
        }
    }
}

impl<R: Resource> ResourceCustomizer<R> {
    pub fn namespace(mut self, namespace: &'static str) -> Self {
        self.namespace = namespace;
        self
    }

    pub fn customize(
        mut self,
        cust: impl Fn(&mut R::Customizer, &R::Config) -> Result<(), PropertyError> + 'static,
    ) -> Self {
        self.customizer = Box::new(cust);
        self
    }
}

/// This macro wraps resource init function.
#[macro_export]
macro_rules! initialize {
    ($env:ident . $ty:ty, $ns:expr, $cu:block) => {
        $env.init_by_namespace_and_customizer::<$ty, Box<
            dyn Fn(
                &mut <$ty as Resource>::Customizer,
                &<$ty as Resource>::Config,
            ) -> Result<(), PropertyError>,
        >>($ns, Box::new($cu))
    };

    ($env:ident . $ty:ty, $cu:block) => {
        initialize!($env.$ty, "", $cu)
    };
    ($env:ident . $ty:ty, $ns:expr) => {
        initialize!($env.$ty, $ns, { |_, _| Ok(()) })
    };
    ($env:ident . $ty:ty) => {
        initialize!($env.$ty, "")
    };
}

impl Resource for () {
    type Config = ();
    type Customizer = ();
    fn create(_config: Self::Config, _customizer: Self::Customizer) -> Result<Self, PropertyError> {
        Ok(())
    }
}
type A = ();
type B = ();
type C = ();

macro_rules! resource_define {
  ($x:ident {$($f:ident:$ty:ty $(,$e:tt)*)+}) => {
    /// hello
    #[allow(missing_debug_implementations)]
    pub struct $x {
      $($f: $ty,)+
    }

    impl $x {
      fn new(env: &impl Application) -> Result<Self, PropertyError> {
           Ok(Self{
$($f: initialize!(env.$ty $(,$e)*)?,)+
           })
      }
    }

  }
}

fn hello() {
    resource_define!(
  Env{
  c: C, ""
  b: B, {|_,_|Ok(())}
  a: A, "", {|_,_|Ok(())}
  } );
}
#[cfg_attr(docsrs, doc(cfg(feature = "app")))]
/// Lazy resource.
#[allow(missing_debug_implementations)]
pub struct Lazy<'a, T> {
    val: Box<dyn FnOnce() -> Result<T, PropertyError> + 'a>,
}

impl<'a, R: Resource> Resource for Lazy<'a, R>
where
    <R as Resource>::Customizer: 'a,
    <R as Resource>::Config: 'a,
{
    type Config = R::Config;
    type Customizer = R::Customizer;

    fn create(config: Self::Config, customizer: Self::Customizer) -> Result<Self, PropertyError> {
        Ok(Lazy {
            val: Box::new(move || R::create(config, customizer)),
        })
    }
}

impl<T> Lazy<'_, T> {
    /// Apply lazy init.
    pub fn apply(self) -> Result<T, PropertyError> {
        (self.val)()
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

    #[test]
    fn resource_test() {
        let env = Salak::builder()
            .set("config.name", "First")
            .set("config.lazy.name", "Second")
            .build()
            .unwrap();
        let a = env.init_with_namespace::<Lazy<'_, A>>("lazy").unwrap();
        let _ = env.init::<A>().unwrap();
        let _ = a.apply().unwrap();
    }

    #[test]
    fn macro_test() {
        let env = Salak::builder()
            .set("config.name", "First")
            .set("config.lazy.name", "Second")
            .build()
            .unwrap();
        let _a = initialize!(env.A, "", { |_, _| Ok(()) });
        let _a = initialize!(env.A, "");
        let _a = initialize!(env.A, { |_, _| Ok(()) });
    }
}
