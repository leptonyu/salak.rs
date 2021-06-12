//! Postgresql connection pool resource.
use native_tls::{Certificate, TlsConnector};
use postgres::{
    config::{ChannelBinding, TargetSessionAttrs},
    error::DbError,
    Client, Config, Error, NoTls,
};
use postgres_native_tls::MakeTlsConnector;
use r2d2::{ManageConnection, Pool};
use salak::{wrapper::NonEmptyVec, *};
#[allow(unused_imports)]
use std::{
    ops::{Deref, DerefMut},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

#[cfg(feature = "metric")]
use crate::metric::{Key, Metric};

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
    #[salak(desc = "Host list")]
    host: NonEmptyVec<String>,
    #[salak(desc = "Port")]
    port: Option<u16>,
    #[salak(default = "postgres", desc = "Username")]
    user: String,
    #[salak(desc = "Password")]
    password: Option<String>,
    #[salak(desc = "Database name")]
    dbname: Option<String>,
    #[salak(desc = "Database options")]
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
    ssl: Option<PostgresSslConfig>,
    pool: PoolConfig,
}

/// Postgresql ssl configuration.
#[cfg_attr(docsrs, doc(cfg(feature = "postgresql")))]
#[derive(FromEnvironment, Debug)]
pub struct PostgresSslConfig {
    cert_path: PathBuf,
}

impl_enum_property!(WrapEnum<ChannelBinding> {
    "disable" => WrapEnum(ChannelBinding::Disable)
    "prefer" => WrapEnum(ChannelBinding::Prefer)
    "require" => WrapEnum(ChannelBinding::Require)
});

enum Tls {
    Noop(NoTls),
    Native(MakeTlsConnector),
}

impl Tls {
    fn new(config: &Option<PostgresSslConfig>) -> Result<Self, PropertyError> {
        Ok(match config {
            Some(ssl) => {
                let body = std::fs::read(&ssl.cert_path)?;
                let cert = Certificate::from_pem(&body)?;
                Tls::Native(MakeTlsConnector::new(
                    TlsConnector::builder().add_root_certificate(cert).build()?,
                ))
            }
            _ => Tls::Noop(NoTls),
        })
    }
}

/// Postgres manage connection smart pointer.
#[allow(missing_debug_implementations)]
#[cfg_attr(docsrs, doc(cfg(feature = "postgresql")))]
pub struct PostgresConnectionManager {
    config: Config,
    tls_connector: Tls,
    #[cfg(feature = "metric")]
    metric: Arc<Metric>,
    #[cfg(feature = "metric")]
    try_count: Key,
    #[cfg(feature = "metric")]
    fail_count: Key,
}

impl ManageConnection for PostgresConnectionManager {
    type Connection = Client;
    type Error = Error;

    fn connect(&self) -> Result<Client, Error> {
        #[cfg(feature = "metric")]
        {
            self.metric.increment_counter(&self.try_count, 1);
        }
        let v = match &self.tls_connector {
            Tls::Noop(_) => self.config.connect(NoTls),
            Tls::Native(v) => self.config.connect(v.clone()),
        };
        match v {
            Ok(client) => Ok(client),
            Err(err) => {
                #[cfg(feature = "metric")]
                {
                    self.metric.increment_counter(&self.fail_count, 1);
                }
                Err(err)
            }
        }
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

/// Postgresql connection thread pool.
#[allow(missing_debug_implementations)]
#[cfg_attr(docsrs, doc(cfg(feature = "postgresql")))]
pub struct PostgresPool(Pool<PostgresConnectionManager>);

impl Deref for PostgresPool {
    type Target = Pool<PostgresConnectionManager>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Postgres Customizer.
#[allow(missing_debug_implementations)]
#[cfg_attr(docsrs, doc(cfg(feature = "postgresql")))]
pub struct PostgresCustomizer {
    /// Sets the notice callback.
    pub(crate) notice_callback: Option<Box<dyn Fn(DbError) + Sync + Send>>,
    /// Set pool customizer.
    pub(crate) pool: PoolCustomizer<PostgresConnectionManager>,
}

impl_pool_ref!(PostgresCustomizer.pool = PostgresConnectionManager);

impl Resource for PostgresPool {
    type Config = PostgresConfig;
    type Customizer = PostgresCustomizer;

    fn create(
        conf: Self::Config,
        _cxt: &FactoryContext<'_>,
        customizer: impl FnOnce(&mut Self::Customizer, &Self::Config) -> Result<(), PropertyError>,
    ) -> Result<Self, PropertyError> {
        let tls_connector = Tls::new(&conf.ssl)?;
        let mut customize = PostgresCustomizer {
            notice_callback: None,
            pool: PoolCustomizer::new(),
        };
        (customizer)(&mut customize, &conf)?;
        let mut config = postgres::Config::new();
        config.user(&conf.user);
        set_option_field!(conf, config, password);
        set_option_field!(conf, config, &, dbname);
        set_option_field!(conf, config, &, options);
        set_option_field!(conf, config, &, application_name);
        for host in conf.host.iter() {
            config.host(host);
        }
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

        #[cfg(feature = "log")]
        log::info!(
            "Postgres at [{}] hosts are {:?}",
            _cxt.current_namespace(),
            config.get_hosts()
        );

        let m = PostgresConnectionManager {
            config,
            tls_connector,
            #[cfg(feature = "metric")]
            metric: _cxt.get_resource()?,
            #[cfg(feature = "metric")]
            try_count: "postgres_connection_try_count".into(),
            #[cfg(feature = "metric")]
            fail_count: "postgres_connection_fail_count".into(),
        };

        Ok(PostgresPool(conf.pool.build_pool(m, customize.pool)?))
    }
}

impl PostgresCustomizer {
    /// Configure notice callback
    pub fn configure_notice_callback(&mut self, handler: impl Fn(DbError) + Sync + Send + 'static) {
        self.notice_callback = Some(Box::new(handler))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn postgres_tests() {
        let env = Salak::builder()
            .set("postgresql.host[0]", "localhost")
            .build()
            .unwrap();
        let pool = env.init_resource::<PostgresPool>();
        assert_eq!(true, pool.is_ok());
    }
}
