//! Postgresql configuration.
use postgres::{
    config::{ChannelBinding, TargetSessionAttrs},
    error::DbError,
    tls::{MakeTlsConnect, TlsConnect},
    Client, Config, Error, NoTls, Socket,
};
use r2d2::{ManageConnection, Pool};
use salak::*;
use std::time::Duration;

use crate::{
    pool::{PoolConfig, PoolCustomizer},
    Buildable, WrapEnum,
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
    application_name: Option<String>,
    #[salak(default = "1s")]
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

/// Postgres connection pool configuration.
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

/// Postgres Customizer
#[allow(missing_debug_implementations)]
#[cfg_attr(docsrs, doc(cfg(feature = "postgresql")))]
pub struct PostgresCustomizer {
    /// Sets the notice callback.
    pub notice_callback: Option<Box<dyn Fn(DbError) + Sync + Send>>,
    /// Set pool customizer.
    pub pool: PoolCustomizer<PostgresConnectionManager<NoTls>>,
}

impl Default for PostgresCustomizer {
    fn default() -> Self {
        PostgresCustomizer {
            notice_callback: None,
            pool: PoolCustomizer::default(),
        }
    }
}

impl Buildable for PostgresConfig {
    type Product = Pool<PostgresConnectionManager<NoTls>>;

    type Customizer = PostgresCustomizer;

    fn build_with_key(
        self,
        _: &impl Environment,
        customizer: Self::Customizer,
    ) -> Result<Self::Product, PropertyError> {
        let mut config = match self.url {
            Some(url) => std::str::FromStr::from_str(&url)?,
            None => postgres::Config::new(),
        };
        set_option_field!(self, config, &, user);
        set_option_field!(self, config, password);
        set_option_field!(self, config, &, dbname);
        set_option_field!(self, config, &, options);
        set_option_field!(self, config, &, application_name);
        set_option_field!(self, config, &, host);
        set_option_field!(self, config, port);
        set_option_field!(self, config, connect_timeout);
        set_option_field!(self, config, keepalives);
        set_option_field!(self, config, keepalives_idle);
        set_option_field!(customizer, config, notice_callback);

        if self.must_allow_write {
            config.target_session_attrs(TargetSessionAttrs::ReadWrite);
        } else {
            config.target_session_attrs(TargetSessionAttrs::Any);
        }

        if let Some(channel_binding) = self.channel_binding {
            config.channel_binding(channel_binding.0);
        }

        let m = PostgresConnectionManager {
            config,
            tls_connector: NoTls,
        };
        self.pool.build_pool(m, customizer.pool)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn postgres_tests() {
        let env = Salak::new().unwrap();
        let pool = env.get::<PostgresConfig>();
        assert_eq!(true, pool.is_ok());
    }
}
