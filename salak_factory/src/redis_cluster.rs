//! Redis cluster connection pool resource.
use crate::pool::{PoolConfig, PoolCustomizer};
use ::redis::cluster::*;
use ::redis::*;
use r2d2::{ManageConnection, Pool};
use salak::*;
use std::ops::Deref;
use std::{str::FromStr, time::Duration};

/// Redis Connection Pool Configuration.
///
/// |property|required|default|
/// |-|-|-|
/// |redis.cluster.url|true||
/// |redis.cluster.password|false||
/// |redis.cluster.readonly|false||
/// |redis.cluster.read_timeout|false||
/// |redis.cluster.write_timeout|false||
/// |redis.cluster.auto_reconnect|false||
/// |redis.cluster.pool.max_size|false|${pool.max_size:}|
/// |redis.cluster.pool.min_idle|false|${pool.min_idle:}|
/// |redis.cluster.pool.thread_name|false|${pool.thread_name:}|
/// |redis.cluster.pool.thread_nums|false|${pool.thread_nums:}|
/// |redis.cluster.pool.test_on_check_out|false|${pool.test_on_check_out:}|
/// |redis.cluster.pool.max_lifetime|false|${pool.max_lifetime:}|
/// |redis.cluster.pool.idle_timeout|false|${pool.idle_timeout:}|
/// |redis.cluster.pool.connection_timeout|false|${pool.connection_timeout:5s}|
/// |redis.cluster.pool.wait_for_init|false|${pool.wait_for_init:false}|
#[cfg_attr(docsrs, doc(cfg(feature = "redis_cluster")))]
#[derive(FromEnvironment, Debug)]
#[salak(prefix = "redis.cluster")]
pub struct RedisClusterConfig {
    url: wrapper::NonEmptyVec<String>,
    password: Option<String>,
    readonly: Option<bool>,
    read_timeout: Option<Duration>,
    write_timeout: Option<Duration>,
    auto_reconnect: Option<bool>,
    pool: PoolConfig,
}

/// Redis manage connection.
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

/// Redis cluster connection pool.
#[allow(missing_debug_implementations)]
pub struct RedisClusterPool(Pool<RedisClusterConnectionManager>);

impl Deref for RedisClusterPool {
    type Target = Pool<RedisClusterConnectionManager>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Resource for RedisClusterPool {
    type Config = RedisClusterConfig;
    type Customizer = PoolCustomizer<RedisClusterConnectionManager>;

    fn create(
        conf: Self::Config,
        _: &impl Factory,
        customizer: impl FnOnce(&mut Self::Customizer, &Self::Config) -> Result<(), PropertyError>,
    ) -> Result<Self, PropertyError> {
        let mut customize = PoolCustomizer::new();
        (customizer)(&mut customize, &conf)?;
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

        Ok(RedisClusterPool(conf.pool.build_pool(
            RedisClusterConnectionManager {
                client,
                read_timeout: conf.read_timeout,
                write_timeout: conf.write_timeout,
                auto_reconnect: conf.auto_reconnect,
            },
            customize,
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
        let pool = env.init_resource::<RedisClusterPool>();
        assert_eq!(true, pool.is_ok());
    }
}
