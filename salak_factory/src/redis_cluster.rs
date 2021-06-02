//! Redis cluster configuration.
use crate::pool::{PoolConfig, PoolCustomizer};
use ::redis::cluster::*;
use ::redis::*;
use r2d2::{ManageConnection, Pool};
use salak::*;
use std::{str::FromStr, time::Duration};

/// Redis Connection Pool Configuration.
///
/// |property|required|default|
/// |-|-|-|
/// |redis_cluster.url|true||
/// |redis_cluster.password|false||
/// |redis_cluster.readonly|false||
/// |redis_cluster.read_timeout|false||
/// |redis_cluster.write_timeout|false||
/// |redis_cluster.auto_reconnect|false||
/// |redis_cluster.pool.max_size|false|${pool.max_size:}|
/// |redis_cluster.pool.min_idle|false|${pool.min_idle:}|
/// |redis_cluster.pool.thread_name|false|${pool.thread_name:}|
/// |redis_cluster.pool.thread_nums|false|${pool.thread_nums:}|
/// |redis_cluster.pool.test_on_check_out|false|${pool.test_on_check_out:}|
/// |redis_cluster.pool.max_lifetime|false|${pool.max_lifetime:}|
/// |redis_cluster.pool.idle_timeout|false|${pool.idle_timeout:}|
/// |redis_cluster.pool.connection_timeout|false|${pool.connection_timeout:5s}|
/// |redis_cluster.pool.wait_for_init|false|${pool.wait_for_init:false}|
#[cfg_attr(docsrs, doc(cfg(feature = "redis_cluster")))]
#[derive(FromEnvironment, Debug)]
#[salak(prefix = "redis_cluster")]
pub struct RedisClusterConfig {
    url: wrapper::NonEmptyVec<String>,
    password: Option<String>,
    readonly: Option<bool>,
    read_timeout: Option<Duration>,
    write_timeout: Option<Duration>,
    auto_reconnect: Option<bool>,
    pool: PoolConfig,
}

/// Redis connection manager
#[cfg_attr(docsrs, doc(cfg(feature = "redis_cluster")))]
#[allow(missing_debug_implementations)]
pub struct RedisClusterConnectionManager {
    client: ClusterClient,
    read_timeout: Option<Duration>,
    write_timeout: Option<Duration>,
    auto_reconnect: Option<bool>,
}

impl ManageConnection for RedisClusterConnectionManager {
    type Connection = ClusterConnection;
    type Error = RedisError;

    fn connect(&self) -> Result<Self::Connection, Self::Error> {
        let conn = self.client.get_connection()?;
        if let Some(auto_reconnect) = self.auto_reconnect {
            conn.set_auto_reconnect(auto_reconnect);
        }
        conn.set_read_timeout(self.read_timeout)?;
        conn.set_write_timeout(self.write_timeout)?;
        Ok(conn)
    }

    fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        cmd("PING").query(conn)
    }

    fn has_broken(&self, conn: &mut Self::Connection) -> bool {
        !conn.check_connection()
    }
}

/// XXX
#[allow(missing_debug_implementations)]
pub struct RedisPool(Pool<RedisClusterConnectionManager>);

impl Resource for RedisPool {
    type Customizer = PoolCustomizer<RedisClusterConnectionManager>;

    type Config = RedisClusterConfig;

    fn create(conf: Self::Config, customizer: Self::Customizer) -> Result<Self, PropertyError> {
        let mut config = vec![];
        for url in conf.url {
            config.push(ConnectionInfo::from_str(&url)?)
        }
        let mut builder = ClusterClientBuilder::new(config);
        if let Some(password) = conf.password {
            builder = builder.password(password);
        }
        if let Some(readonly) = conf.readonly {
            builder = builder.readonly(readonly);
        }
        let client = builder.open()?;

        Ok(RedisPool(conf.pool.build_pool(
            RedisClusterConnectionManager {
                client,
                read_timeout: conf.read_timeout,
                write_timeout: conf.write_timeout,
                auto_reconnect: conf.auto_reconnect,
            },
            customizer,
        )?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn redis_tests() {
        let env = Salak::builder()
            .set("redis_cluster.url[0]", "redis://127.0.0.1/")
            .build()
            .unwrap();
        let pool = env.get::<RedisClusterConfig>();
        assert_eq!(true, pool.is_ok());
    }
}
