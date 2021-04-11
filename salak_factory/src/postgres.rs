use super::*;
use ::postgres::{
    config::{ChannelBinding, TargetSessionAttrs},
    error::DbError,
    tls::{MakeTlsConnect, TlsConnect},
    Client, Config, Error, NoTls, Socket,
};
use std::time::Duration;

/// Postgres Connection Pool Configuration.
#[derive(FromEnvironment, Debug)]
pub struct PostgresConfig {
    #[salak(default = "postgresql://postgres@localhost")]
    url: Option<String>,
    host: Option<String>,
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
    channel_binding: Option<String>,
    pool: PoolConfig,
}

/// Postgres connection pool configuration.
#[derive(Debug)]
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

    fn prefix() -> &'static str {
        "postgresql"
    }

    fn build_with_key(
        self,
        _: &impl Environment,
        customizer: Self::Customizer,
    ) -> Result<Self::Product, PropertyError> {
        let mut config = match self.url {
            Some(url) => std::str::FromStr::from_str(&url)
                .map_err(|e| PropertyError::ParseFail(format!("{}", e)))?,
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
            config.channel_binding(match &channel_binding.to_lowercase()[..] {
                "disable" => Ok(ChannelBinding::Disable),
                "prefer" => Ok(ChannelBinding::Prefer),
                "require" => Ok(ChannelBinding::Require),
                _ => Err(PropertyError::parse_failed("Invalid ChannelBinding")),
            }?);
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
        let env = Salak::new().build();
        let pool = env.build::<PostgresConfig>();
        assert_eq!(true, pool.is_ok());
    }
}
