//! Env logger
use log::*;
use salak::*;

/// Logger config.
#[derive(FromEnvironment, Debug)]
#[salak(prefix = "logger")]
#[allow(missing_copy_implementations)]
pub struct LogConfig {
    #[salak(default = "info")]
    max_level: LevelFilter,
}

/// Logger resource.
#[allow(missing_debug_implementations, missing_copy_implementations)]
pub struct Logger;

impl Resource for Logger {
    type Config = LogConfig;
    type Customizer = ();

    fn create(
        c: Self::Config,
        _: &FactoryContext<'_>,
        _: impl FnOnce(&mut Self::Customizer, &Self::Config) -> Result<(), PropertyError>,
    ) -> Result<Self, PropertyError> {
        env_logger::builder().filter_level(c.max_level).try_init()?;
        Ok(Logger)
    }

    fn order() -> Ordered {
        PRIORITY_HIGHEST
    }
}
