//! Generic pool configuration.

use r2d2::{CustomizeConnection, HandleError, HandleEvent};
pub(crate) use r2d2::{ManageConnection, Pool};
use scheduled_thread_pool::ScheduledThreadPool;

use crate::metric::AnyKey;

use super::*;
pub(crate) use std::time::Duration;

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
    ($y: expr, $config: expr, $x: tt) => {
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

impl PoolConfig {
    pub(crate) fn build_pool<M: ManageConnection>(
        self,
        m: M,
        customize: PoolCustomizer<M>,
    ) -> Result<Pool<M>, PropertyError> {
        let thread_nums = self.thread_nums.unwrap_or(3);
        let mut build: r2d2::Builder<M> = Pool::builder()
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
        use crate::metric::Metric;
        let metric = factory.get_resource::<Metric>()?;
        let pool = pool.clone();
        let namespace = factory.current_namespace();
        metric.add_listen_state(move |env| {
            let state = pool.state();
            env.gauge(
                K::new_key("idle_thread_count", namespace),
                state.idle_connections as f64,
            );
            env.gauge(
                K::new_key("thread_count", namespace),
                state.connections as f64,
            );
            Ok(())
        });
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
