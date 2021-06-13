//! Generic pool configuration.

use r2d2::{CustomizeConnection, HandleError, HandleEvent};
pub(crate) use r2d2::{ManageConnection, Pool};
use scheduled_thread_pool::ScheduledThreadPool;

#[cfg(feature = "metric")]
use crate::metric::{AnyKey, GaugeValue, Key, Label, Metric, Unit};

use super::*;
pub(crate) use std::time::Duration;
#[allow(unused_imports)]
use std::{ops::Deref, sync::Arc};

/// Generic Pool Configuration.
#[cfg_attr(docsrs, doc(cfg(feature = "pool")))]
#[derive(FromEnvironment, Debug)]
pub struct PoolConfig {
    #[salak(
        default = "${pool.max_size:5}",
        desc = "The maximum number of connections."
    )]
    max_size: Option<u32>,
    #[salak(
        default = "${pool.min_idle:1}",
        desc = "The minimum idle connection count."
    )]
    min_idle: Option<u32>,
    #[salak(default = "${pool.thread_name:}", desc = "Pool thread name.")]
    thread_name: Option<String>,
    #[salak(default = "${pool.thread_nums:}", desc = "Pool thread size.")]
    thread_nums: Option<usize>,
    #[salak(
        default = "${pool.test_on_check_out:}",
        desc = "Test connection on check out."
    )]
    test_on_check_out: Option<bool>,
    #[salak(
        default = "${pool.max_lifetime:}",
        desc = "Maximum connection lifetime."
    )]
    max_lifetime: Option<Duration>,
    #[salak(
        default = "${pool.idle_timeout:}",
        desc = "Idle connections keep time."
    )]
    idle_timeout: Option<Duration>,
    #[salak(
        default = "${pool.connection_timeout:1s}",
        desc = "Connection timeout."
    )]
    connection_timeout: Option<Duration>,
    #[salak(
        default = "${pool.wait_for_init:false}",
        desc = "Wait for init when start pool."
    )]
    wait_for_init: bool,
}

macro_rules! set_option_field_return {
    ($y: ident, $config: ident, $x: tt) => {
        if let Some($x) = $y.$x {
            $config = $config.$x($x);
        }
    };
}

/// PoolCustomizer
#[allow(missing_debug_implementations)]
#[cfg_attr(docsrs, doc(cfg(feature = "pool")))]
pub struct PoolCustomizer<M: ManageConnection> {
    /// Error handler
    pub(crate) error_handler: Option<Box<dyn HandleError<M::Error>>>,
    /// Event handler
    pub(crate) event_handler: Option<Box<dyn HandleEvent>>,
    /// Connection customizer
    pub(crate) connection_customizer: Option<Box<dyn CustomizeConnection<M::Connection, M::Error>>>,
}

impl<M: ManageConnection> PoolCustomizer<M> {
    pub(crate) fn new() -> Self {
        Self {
            error_handler: None,
            event_handler: None,
            connection_customizer: None,
        }
    }
}

impl<M: ManageConnection> PoolCustomizer<M> {
    /// Configure error handler.
    pub fn configure_error_handler(&mut self, handler: impl HandleError<M::Error>) {
        self.error_handler = Some(Box::new(handler));
    }
    /// Configure event handler.
    pub fn configure_event_handler(&mut self, handler: impl HandleEvent + 'static) {
        self.event_handler = Some(Box::new(handler));
    }
    /// Configure connection customizer.
    pub fn configure_connection_customizer(
        &mut self,
        handler: impl CustomizeConnection<M::Connection, M::Error>,
    ) {
        self.connection_customizer = Some(Box::new(handler));
    }
}

/// Wrapper for connection.
#[allow(missing_debug_implementations)]
pub struct ManagedConnection<M> {
    inner: M,
    #[cfg(feature = "metric")]
    try_count: Key,
    #[cfg(feature = "metric")]
    fail_count: Key,
    #[cfg(feature = "metric")]
    latency: Key,
    #[cfg(feature = "metric")]
    metric: Option<Arc<Metric>>,
}

impl<M: ManageConnection> ManageConnection for ManagedConnection<M> {
    type Connection = M::Connection;

    type Error = M::Error;

    fn connect(&self) -> Result<Self::Connection, Self::Error> {
        #[cfg(feature = "metric")]
        if let Some(metric) = &self.metric {
            let now = std::time::SystemTime::now();
            metric.increment_counter(&self.try_count, 1);
            // metric.increment_counter(key, value);
            let v = match self.inner.connect() {
                Ok(v) => Ok(v),
                Err(err) => {
                    metric.increment_counter(&self.fail_count, 1);
                    Err(err)
                }
            };
            if let Ok(d) = std::time::SystemTime::now().duration_since(now) {
                metric.update_gauge(&self.latency, GaugeValue::Increment(d.as_micros() as f64));
            }
            v
        } else {
            self.inner.connect()
        }
        #[cfg(not(feature = "metric"))]
        self.inner.connect()
    }

    fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        self.inner.is_valid(conn)
    }

    fn has_broken(&self, conn: &mut Self::Connection) -> bool {
        self.inner.has_broken(conn)
    }
}

impl<M: ManageConnection> Deref for ManagedConnection<M> {
    type Target = M;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl PoolConfig {
    pub(crate) fn build_pool<M: ManageConnection>(
        self,
        _context: &FactoryContext<'_>,
        m: M,
        customize: PoolCustomizer<M>,
    ) -> Result<Pool<ManagedConnection<M>>, PropertyError> {
        let thread_nums = self.thread_nums.unwrap_or(3);
        let mut build: r2d2::Builder<ManagedConnection<M>> = Pool::builder()
            .min_idle(self.min_idle)
            .max_lifetime(self.max_lifetime)
            .idle_timeout(self.idle_timeout)
            .thread_pool(std::sync::Arc::new(match self.thread_name {
                Some(name) => ScheduledThreadPool::with_name(&name, thread_nums),
                None => ScheduledThreadPool::new(thread_nums),
            }));
        set_option_field_return!(self, build, connection_timeout);
        set_option_field_return!(self, build, max_size);
        set_option_field_return!(self, build, test_on_check_out);
        set_option_field_return!(customize, build, error_handler);
        set_option_field_return!(customize, build, event_handler);
        set_option_field_return!(customize, build, connection_customizer);

        #[cfg(feature = "metric")]
        let namespace = if _context.current_namespace().is_empty() {
            "default"
        } else {
            _context.current_namespace()
        };

        let m = ManagedConnection {
            inner: m,
            #[cfg(feature = "metric")]
            try_count: Key::from_parts(
                "thread_pool.connection.try_count",
                vec![Label::new("namespace", namespace)],
            ),
            #[cfg(feature = "metric")]
            fail_count: Key::from_parts(
                "thread_pool.connection.fail_count",
                vec![Label::new("namespace", namespace)],
            ),
            #[cfg(feature = "metric")]
            latency: Key::from_parts(
                "thread_pool.connection.latency",
                vec![Label::new("namespace", namespace)],
            ),
            #[cfg(feature = "metric")]
            metric: _context.get_optional_resource()?,
        };

        #[cfg(feature = "metric")]
        if let Some(metric) = &m.metric {
            metric.register_gauge(&m.latency, Some(Unit::Microseconds), None);
        }

        if self.wait_for_init {
            Ok(build.build(m)?)
        } else {
            Ok(build.build_unchecked(m))
        }
    }

    #[cfg(feature = "metric")]
    pub(crate) fn post_pool_initialized_and_registered<M: ManageConnection, K: AnyKey>(
        pool: &Pool<M>,
        factory: &FactoryContext<'_>,
    ) -> Result<(), PropertyError> {
        if let Some(metric) = factory.get_optional_resource::<Metric>()? {
            let pool = pool.clone();
            let namespace = factory.current_namespace();
            metric.gauge(
                K::new_key("thread_pool.max_count", namespace),
                pool.max_size() as f64,
            );
            if let Some(min) = pool.min_idle() {
                metric.gauge(
                    K::new_key("thread_pool.min_idle_count", namespace),
                    min as f64,
                );
            }
            metric.add_listen_state(move |env| {
                let state = pool.state();
                env.gauge(
                    K::new_key("thread_pool.idle_count", namespace),
                    state.idle_connections as f64,
                );
                env.gauge(
                    K::new_key("thread_pool.active_count", namespace),
                    state.connections as f64,
                );
                Ok(())
            });
        }
        Ok(())
    }
}

#[allow(unused_macros)]
macro_rules! impl_pool_ref {
    ($x:ident.$f:ident = $y:ty) => {
        impl Deref for $x {
            type Target = PoolCustomizer<$y>;

            fn deref(&self) -> &Self::Target {
                &self.$f
            }
        }

        impl DerefMut for $x {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.$f
            }
        }
    };
}
