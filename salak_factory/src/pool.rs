use r2d2::{CustomizeConnection, HandleError, HandleEvent};
pub(crate) use r2d2::{ManageConnection, Pool};
use scheduled_thread_pool::ScheduledThreadPool;

use super::*;
pub(crate) use std::time::Duration;

/// Generic Pool Configuration.
#[derive(FromEnvironment, Debug)]
pub struct PoolConfig {
    #[salak(default = "${pool.max_size:}")]
    max_size: Option<u32>,
    #[salak(default = "${pool.min_idle:}")]
    min_idle: Option<u32>,
    #[salak(default = "${pool.thread_name:}")]
    thread_name: Option<String>,
    #[salak(default = "${pool.thread_nums:}")]
    thread_nums: Option<usize>,
    #[salak(default = "${pool.test_on_check_out:}")]
    test_on_check_out: Option<bool>,
    #[salak(default = "${pool.max_lifetime:}")]
    max_lifetime: Option<Duration>,
    #[salak(default = "${pool.idle_timeout:}")]
    idle_timeout: Option<Duration>,
    #[salak(default = "${pool.connection_timeout:5s}")]
    connection_timeout: Option<Duration>,
    #[salak(default = "false")]
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
pub struct PoolCustomizer<M: ManageConnection> {
    /// Error handler
    pub error_handler: Option<Box<dyn HandleError<M::Error>>>,
    /// Event handler
    pub event_handler: Option<Box<dyn HandleEvent>>,
    /// Connection customizer
    pub connection_customizer: Option<Box<dyn CustomizeConnection<M::Connection, M::Error>>>,
}

impl<M: ManageConnection> Default for PoolCustomizer<M> {
    fn default() -> Self {
        PoolCustomizer {
            error_handler: None,
            event_handler: None,
            connection_customizer: None,
        }
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
            build
                .build(m)
                .map_err(|e| PropertyError::parse_failed(&format!("{}", e)))
        } else {
            Ok(build.build_unchecked(m))
        }
    }
}
