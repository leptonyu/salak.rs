use super::*;
use ::redis::cluster::*;
use ::redis::*;
use std::str::FromStr;

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
#[derive(FromEnvironment, Debug)]
pub struct RedisClusterConfig {
    #[salak(required = true)]
    url: Vec<String>,
    password: Option<String>,
    readonly: Option<bool>,
    read_timeout: Option<Duration>,
    write_timeout: Option<Duration>,
    auto_reconnect: Option<bool>,
    pool: PoolConfig,
}

/// Redis connection manager
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

impl Buildable for RedisClusterConfig {
    type Product = Pool<RedisClusterConnectionManager>;
    type Customizer = PoolCustomizer<RedisClusterConnectionManager>;

    fn prefix() -> &'static str {
        "redis_cluster"
    }

    fn build_with_key(
        self,
        _: &impl Environment,
        customizer: Self::Customizer,
    ) -> Result<Self::Product, PropertyError> {
        let mut config = vec![];
        for url in self.url {
            config.push(
                ConnectionInfo::from_str(&url)
                    .map_err(|e| PropertyError::ParseFail(format!("{}", e)))?,
            )
        }
        if config.is_empty() {
            return Err(PropertyError::ParseFail(format!(
                "{}.url not set",
                Self::prefix()
            )));
        }
        let mut builder = ClusterClientBuilder::new(config);
        if let Some(password) = self.password {
            builder = builder.password(password);
        }
        if let Some(readonly) = self.readonly {
            builder = builder.readonly(readonly);
        }
        let client = builder
            .open()
            .map_err(|e| PropertyError::ParseFail(format!("{}", e)))?;

        self.pool.build_pool(
            RedisClusterConnectionManager {
                client,
                read_timeout: self.read_timeout,
                write_timeout: self.write_timeout,
                auto_reconnect: self.auto_reconnect,
            },
            customizer,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn redis_tests() {
        let env = Salak::new()
            .set_property("redis_cluster.url[0]", "redis://127.0.0.1/")
            .build();
        let pool = env.build::<RedisClusterConfig>();
        assert_eq!(true, pool.is_ok());

        print_keys::<RedisClusterConfig>();
    }
}
