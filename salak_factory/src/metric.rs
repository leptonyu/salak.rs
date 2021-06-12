//! Metric with prometheus
use core::f64;
pub use metrics::*;
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle, PrometheusRecorder};
use parking_lot::Mutex;
use salak::*;
use std::{
    net::SocketAddr,
    ops::Deref,
    sync::Arc,
    thread::sleep,
    time::{Duration, UNIX_EPOCH},
};

/// Metric recorder.
#[allow(missing_debug_implementations, missing_copy_implementations)]
pub struct Metric(
    PrometheusRecorder,
    Mutex<Vec<Box<dyn Fn(&Metric) -> Result<(), PropertyError> + Send + 'static>>>,
);

impl Deref for Metric {
    type Target = dyn Recorder;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Metric {
    /// Get prometheus handle
    pub fn handle(&self) -> PrometheusHandle {
        self.0.handle()
    }

    /// Update gauge.
    pub fn gauge<K: Into<Key>>(&self, k: K, val: f64) {
        self.0.update_gauge(&k.into(), GaugeValue::Absolute(val));
    }

    /// Increment count.
    pub fn count_inc<K: Into<Key>>(&self, k: K, val: u64) {
        self.0.increment_counter(&k.into(), val);
    }

    /// Add listen state.
    pub fn add_listen_state(
        &self,
        listen: impl Fn(&Self) -> Result<(), PropertyError> + Send + 'static,
    ) {
        let mut guard = self.1.lock();
        guard.push(Box::new(listen));
    }
}

/// Metric configuration.
#[derive(FromEnvironment, Debug)]
#[salak(prefix = "metric")]
pub struct MetricConfig {
    #[salak(default = "${salak.app.name}")]
    application: String,
    #[salak(default = "${salak.app.version}")]
    version: String,
    #[salak(desc = "Metric address, default is :9000")]
    address: Option<SocketAddr>,
}

macro_rules! set_config {
    ($config:expr => $builder:ident.$fn:ident) => {
        if let Some(val) = $config {
            $builder = $builder.$fn(val);
        }
    };
}

impl Resource for Metric {
    type Config = MetricConfig;

    type Customizer = PrometheusBuilder;

    fn create(
        config: Self::Config,
        _factory: &FactoryContext<'_>,
        customizer: impl FnOnce(&mut Self::Customizer, &Self::Config) -> Result<(), PropertyError>,
    ) -> Result<Self, PropertyError> {
        let mut builder = PrometheusBuilder::new();
        (customizer)(&mut builder, &config)?;
        set_config!(config.address => builder.listen_address);

        let x = Metric(builder.build(), Mutex::new(Vec::new()));
        x.gauge(
            "uptime",
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)?
                .as_millis() as f64,
        );
        Ok(x)
    }

    fn order() -> Ordered {
        PRIORITY_HIGH
    }

    fn register_dependent_resources(builder: &mut FactoryBuilder<'_>) -> Result<(), PropertyError> {
        builder.submit(|req: Arc<Metric>| {
            let _h = req.handle();
            loop {
                #[cfg(feature = "log")]
                log::info!("PROMETHES: \n{}", _h.render());
                sleep(Duration::from_secs(5));
            }
        })
    }
}
