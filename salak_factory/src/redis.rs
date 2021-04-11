use super::*;
use ::redis::*;
use std::str::FromStr;

/// Redis Connection Pool Configuration.
#[derive(FromEnvironment, Debug)]
pub struct RedisConfig {
    url: Option<String>,
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
    connect_timeout: Option<Duration>,
    read_timeout: Option<Duration>,
    write_timeout: Option<Duration>,
    pool: PoolConfig,
}

/// Redis connection manager
#[allow(missing_debug_implementations)]
pub struct RedisConnectionManager {
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
        Ok(conn)
    }

    fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        cmd("PING").query(conn)
    }

    fn has_broken(&self, conn: &mut Self::Connection) -> bool {
        !ConnectionLike::is_open(conn)
    }
}

impl Buildable for RedisConfig {
    type Product = Pool<RedisConnectionManager>;
    type Customizer = PoolCustomizer<RedisConnectionManager>;

    fn prefix() -> &'static str {
        "redis"
    }

    fn build_with_key(
        self,
        _: &impl Environment,
        customizer: Self::Customizer,
    ) -> Result<Self::Product, PropertyError> {
        let config = if let Some(url) = self.url {
            ConnectionInfo::from_str(&url)
                .map_err(|e| PropertyError::ParseFail(format!("{}", e)))?
        } else {
            let host = self.host;
            let port = self.port;
            let addr = if self.ssl {
                ConnectionAddr::TcpTls {
                    host,
                    port,
                    insecure: self.ssl_insecure,
                }
            } else {
                ConnectionAddr::Tcp(host, port)
            };
            ConnectionInfo {
                addr: Box::new(addr),
                db: self.db.unwrap_or(0),
                username: self.user,
                passwd: self.password,
            }
        };
        self.pool.build_pool(
            RedisConnectionManager {
                config,
                connect_timeout: self.connect_timeout,
                read_timeout: self.read_timeout,
                write_timeout: self.write_timeout,
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
        let env = Salak::new().build();
        let pool = env.build::<RedisConfig>();
        assert_eq!(true, pool.is_ok());

        let redis_pool = env.build::<RedisConfig>().unwrap();
        let mut redis_conn = redis_pool.get().unwrap();
        let key = "hello";
        let _: u64 = redis_conn.set(key, 1u64).unwrap();

        for (k, o, v) in RedisConfig::list_keys("primary") {
            if let Some(v) = v {
                println!("{}[required={}]: {}", k, o, v);
            } else {
                println!("{}[required={}]", k, o);
            }
        }
    }
}
