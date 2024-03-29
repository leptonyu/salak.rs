//! Single node redis configuratino.
use crate::pool::{ManagedConnection, PoolConfig, PoolCustomizer};
use ::redis::*;
use r2d2::{ManageConnection, Pool};
use salak::*;
#[allow(unused_imports)]
use std::{ops::Deref, sync::Arc, time::Duration};

/// Redis Connection Pool Configuration.
///
/// |property|required|default|
/// |-|-|-|
/// |redis.url|false||
/// |redis.host|false|localhost|
/// |redis.port|false|6379|
/// |redis.ssl|false|false|
/// |redis.ssl_insecure|false|false|
/// |redis.db|false||
/// |redis.user|false||
/// |redis.password|false||
/// |redis.connect_timeout|false||
/// |redis.read_timeout|false||
/// |redis.write_timeout|false||
/// |redis.pool.max_size|false|${pool.max_size:}|
/// |redis.pool.min_idle|false|${pool.min_idle:}|
/// |redis.pool.thread_name|false|${pool.thread_name:}|
/// |redis.pool.thread_nums|false|${pool.thread_nums:}|
/// |redis.pool.test_on_check_out|false|${pool.test_on_check_out:}|
/// |redis.pool.max_lifetime|false|${pool.max_lifetime:}|
/// |redis.pool.idle_timeout|false|${pool.idle_timeout:}|
/// |redis.pool.connection_timeout|false|${pool.connection_timeout:5s}|
/// |redis.pool.wait_for_init|false|${pool.wait_for_init:false}|
#[cfg_attr(docsrs, doc(cfg(feature = "redis_default")))]
#[derive(FromEnvironment, Debug)]
#[salak(prefix = "redis")]
pub struct RedisConfig {
    #[salak(default = "localhost")]
    host: String,
    #[salak(default = "6379")]
    port: u16,
    #[salak(default = "false")]
    ssl: bool,
    #[salak(default = "false")]
    ssl_insecure: bool,
    db: Option<i64>,
    user: Option<String>,
    password: Option<String>,
    #[salak(default = "500ms")]
    connect_timeout: Option<Duration>,
    read_timeout: Option<Duration>,
    write_timeout: Option<Duration>,
    pool: PoolConfig,
}

/// Redis manage connection.
#[cfg_attr(docsrs, doc(cfg(feature = "redis_default")))]
#[allow(missing_debug_implementations)]
pub struct RedisConnectionManager {
    #[allow(dead_code)]
    namespace: &'static str,
    config: ConnectionInfo,
    connect_timeout: Option<Duration>,
    read_timeout: Option<Duration>,
    write_timeout: Option<Duration>,
}

impl ManageConnection for RedisConnectionManager {
    type Connection = Connection;
    type Error = RedisError;

    fn connect(&self) -> Result<Self::Connection, Self::Error> {
        let client = Client::open(self.config.clone())?;
        let conn = match self.connect_timeout {
            Some(du) => client.get_connection_with_timeout(du),
            _ => client.get_connection(),
        }?;
        conn.set_read_timeout(self.read_timeout)?;
        conn.set_write_timeout(self.write_timeout)?;
        #[cfg(feature = "log")]
        log::trace!(
            "Redis [{}] get connection at {}",
            self.namespace,
            self.config.addr
        );
        Ok(conn)
    }

    fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        cmd("PING").query(conn)
    }

    fn has_broken(&self, conn: &mut Self::Connection) -> bool {
        !ConnectionLike::is_open(conn)
    }
}

/// Redis connection pool.
#[allow(missing_debug_implementations)]
#[derive(Clone)]
pub struct RedisPool(Pool<ManagedConnection<RedisConnectionManager>>);

impl Deref for RedisPool {
    type Target = Pool<ManagedConnection<RedisConnectionManager>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Resource for RedisPool {
    type Config = RedisConfig;
    type Customizer = PoolCustomizer<RedisConnectionManager>;

    fn create(
        conf: Self::Config,
        _cxt: &FactoryContext<'_>,
        customizer: impl FnOnce(&mut Self::Customizer, &Self::Config) -> Result<(), PropertyError>,
    ) -> Result<Self, PropertyError> {
        let mut customize = PoolCustomizer::new();
        (customizer)(&mut customize, &conf)?;
        let host = conf.host;
        let port = conf.port;
        let addr = if conf.ssl {
            ConnectionAddr::TcpTls {
                host,
                port,
                insecure: conf.ssl_insecure,
            }
        } else {
            ConnectionAddr::Tcp(host, port)
        };
        let config = ConnectionInfo {
            addr,
            redis: RedisConnectionInfo {
                db: conf.db.unwrap_or(0),
                username: conf.user,
                password: conf.password,
            },
        };
        #[cfg(feature = "log")]
        log::info!(
            "Redis at [{}] url is {:?}",
            _cxt.current_namespace(),
            config.addr
        );
        Ok(RedisPool(conf.pool.build_pool(
            _cxt,
            RedisConnectionManager {
                namespace: _cxt.current_namespace(),
                config,
                connect_timeout: conf.connect_timeout,
                read_timeout: conf.read_timeout,
                write_timeout: conf.write_timeout,
            },
            customize,
        )?))
    }

    #[cfg(feature = "metric")]
    fn post_initialized_and_registered(
        pool: &Arc<Self>,
        factory: &FactoryContext<'_>,
    ) -> Result<(), PropertyError> {
        PoolConfig::post_pool_initialized_and_registered::<
            ManagedConnection<RedisConnectionManager>,
            Self,
        >(pool, factory)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn redis_tests() {
        let env = Salak::new().unwrap();
        let pool = env.init_resource::<RedisPool>();
        assert_eq!(true, pool.is_ok());
    }
}
