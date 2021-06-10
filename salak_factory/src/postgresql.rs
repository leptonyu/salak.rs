//! Postgresql connection pool resource.
use postgres::{
    config::{ChannelBinding, TargetSessionAttrs},
    error::DbError,
    tls::{MakeTlsConnect, TlsConnect},
    Client, Config, Error, NoTls, Socket,
};
use r2d2::{ManageConnection, Pool};
use salak::*;
use std::{
    ops::{Deref, DerefMut},
    time::Duration,
};

use crate::{
    pool::{PoolConfig, PoolCustomizer},
    WrapEnum,
};

/// Postgres Connection Pool Configuration.
///
/// |property|required|default|
/// |-|-|-|
/// |postgresql.url|false|postgresql://postgres@localhost|
/// |postgresql.host|false||
/// |postgresql.port|false||
/// |postgresql.user|false||
/// |postgresql.password|false||
/// |postgresql.dbname|false||
/// |postgresql.options|false||
/// |postgresql.application_name|false||
/// |postgresql.connect_timeout|false|1s|
/// |postgresql.keepalives|false||
/// |postgresql.keepalives_idle|false||
/// |postgresql.must_allow_write|false|true|
/// |postgresql.channel_binding|false||
/// |postgresql.pool.max_size|false|${pool.max_size:}|
/// |postgresql.pool.min_idle|false|${pool.min_idle:}|
/// |postgresql.pool.thread_name|false|${pool.thread_name:}|
/// |postgresql.pool.thread_nums|false|${pool.thread_nums:}|
/// |postgresql.pool.test_on_check_out|false|${pool.test_on_check_out:}|
/// |postgresql.pool.max_lifetime|false|${pool.max_lifetime:}|
/// |postgresql.pool.idle_timeout|false|${pool.idle_timeout:}|
/// |postgresql.pool.connection_timeout|false|${pool.connection_timeout:5s}|
/// |postgresql.pool.wait_for_init|false|${pool.wait_for_init:false}|
#[cfg_attr(docsrs, doc(cfg(feature = "postgresql")))]
#[derive(FromEnvironment, Debug)]
#[salak(prefix = "postgresql")]
pub struct PostgresConfig {
    #[salak(
        default = "postgresql://postgres@localhost",
        desc = "Postgresql url, can reset by host & port."
    )]
    url: Option<String>,
    #[salak(desc = "Postgresql host")]
    host: Option<String>,
    #[salak(desc = "Postgresql port")]
    port: Option<u16>,
    user: Option<String>,
    password: Option<String>,
    dbname: Option<String>,
    options: Option<String>,
    #[salak(default = "${salak.application.name:}")]
    application_name: Option<String>,
    #[salak(default = "500ms")]
    connect_timeout: Option<Duration>,
    keepalives: Option<bool>,
    keepalives_idle: Option<Duration>,
    #[salak(default = "true")]
    must_allow_write: bool,
    #[salak(desc = "disable/prefer/require")]
    channel_binding: Option<WrapEnum<ChannelBinding>>,
    pool: PoolConfig,
}

impl_enum_property!(WrapEnum<ChannelBinding> {
    "disable" => WrapEnum(ChannelBinding::Disable)
    "prefer" => WrapEnum(ChannelBinding::Prefer)
    "require" => WrapEnum(ChannelBinding::Require)
});

/// Postgres manage connection smart pointer.
#[derive(Debug)]
#[cfg_attr(docsrs, doc(cfg(feature = "postgresql")))]
pub struct PostgresConnectionManager<T> {
    config: Config,
    tls_connector: T,
}

impl<T> ManageConnection for PostgresConnectionManager<T>
where
    T: MakeTlsConnect<Socket> + Clone + 'static + Sync + Send,
    T::TlsConnect: Send,
    T::Stream: Send,
    <T::TlsConnect as TlsConnect<Socket>>::Future: Send,
{
    type Connection = Client;
    type Error = Error;

    fn connect(&self) -> Result<Client, Error> {
        self.config.connect(self.tls_connector.clone())
    }

    fn is_valid(&self, client: &mut Client) -> Result<(), Error> {
        client.simple_query("").map(|_| ())
    }

    fn has_broken(&self, client: &mut Client) -> bool {
        client.is_closed()
    }
}

macro_rules! set_option_field {
    ($y: expr, $config: expr, $x: tt) => {
        if let Some($x) = $y.$x {
            $config.$x($x);
        }
    };
    ($y: expr, $config: expr, $x: tt, $z: tt) => {
        if let Some($z) = $y.$z {
            $config.$z($x$z);
        }
    };
}

/// Postgres Customizer.
#[allow(missing_debug_implementations)]
#[cfg_attr(docsrs, doc(cfg(feature = "postgresql")))]
pub struct PostgresCustomizer {
    /// Sets the notice callback.
    notice_callback: Option<Box<dyn Fn(DbError) + Sync + Send>>,
    /// Set pool customizer.
    pool: PoolCustomizer<PostgresConnectionManager<NoTls>>,
}

impl_pool_ref!(PostgresCustomizer.pool = PostgresConnectionManager<NoTls>);

impl PostgresCustomizer {
    /// Configure notice callback
    pub fn configure_notice_callback(&mut self, handler: impl Fn(DbError) + Sync + Send + 'static) {
        self.notice_callback = Some(Box::new(handler))
    }
}

/// Postgresql connection pool.
#[allow(missing_debug_implementations)]
pub struct PostgresPool(Pool<PostgresConnectionManager<NoTls>>);

impl Deref for PostgresPool {
    type Target = Pool<PostgresConnectionManager<NoTls>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Resource for PostgresPool {
    type Config = PostgresConfig;
    type Customizer = PostgresCustomizer;

    fn create(
        conf: Self::Config,
        _: &FactoryContext<'_>,
        customizer: impl FnOnce(&mut Self::Customizer, &Self::Config) -> Result<(), PropertyError>,
    ) -> Result<Self, PropertyError> {
        let mut customize = PostgresCustomizer {
            notice_callback: None,
            pool: PoolCustomizer::new(),
        };
        (customizer)(&mut customize, &conf)?;
        let mut config = match conf.url {
            Some(url) => std::str::FromStr::from_str(&url)?,
            None => postgres::Config::new(),
        };
        set_option_field!(conf, config, &, user);
        set_option_field!(conf, config, password);
        set_option_field!(conf, config, &, dbname);
        set_option_field!(conf, config, &, options);
        set_option_field!(conf, config, &, application_name);
        set_option_field!(conf, config, &, host);
        set_option_field!(conf, config, port);
        set_option_field!(conf, config, connect_timeout);
        set_option_field!(conf, config, keepalives);
        set_option_field!(conf, config, keepalives_idle);
        set_option_field!(customize, config, notice_callback);

        if conf.must_allow_write {
            config.target_session_attrs(TargetSessionAttrs::ReadWrite);
        } else {
            config.target_session_attrs(TargetSessionAttrs::Any);
        }

        if let Some(channel_binding) = conf.channel_binding {
            config.channel_binding(channel_binding.0);
        }

        let m = PostgresConnectionManager {
            config,
            tls_connector: NoTls,
        };
        Ok(PostgresPool(conf.pool.build_pool(m, customize.pool)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn postgres_tests() {
        let env = Salak::new().unwrap();
        let pool = env.init_resource::<PostgresPool>();
        assert_eq!(true, pool.is_ok());
    }
}
