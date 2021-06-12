//! Metric with prometheus
use core::f64;
pub use metrics::*;
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle, PrometheusRecorder};
use parking_lot::Mutex;
use salak::*;
use std::{
    any::Any,
    net::SocketAddr,
    ops::Deref,
    sync::Arc,
    thread::sleep,
    time::{Duration, UNIX_EPOCH},
};
use sysinfo::*;

/// Metric recorder.
#[allow(missing_debug_implementations, missing_copy_implementations)]
pub struct Metric {
    recorder: PrometheusRecorder,
    handle: PrometheusHandle,
    code: Mutex<Vec<Box<dyn Fn(&Metric) -> Result<(), PropertyError> + Send + 'static>>>,
    sys: System,
}

impl Deref for Metric {
    type Target = dyn Recorder;

    fn deref(&self) -> &Self::Target {
        &self.recorder
    }
}

/// Turn any to key.
pub trait AnyKey: Any + Resource {
    /// Create key from name and namespace.
    fn new_key(name: &'static str, namespace: &'static str) -> Key {
        Key::from_parts(
            name,
            vec![
                Label::new(
                    "namespace",
                    if namespace.is_empty() {
                        "default"
                    } else {
                        namespace
                    },
                ),
                Label::new("name", Self::Config::prefix()),
            ],
        )
    }
}

impl<T: Any + Resource> AnyKey for T {}

macro_rules! set_val {
    ($labels:ident:$self:ident.$fn:ident => $val:expr) => {
        if let Some(n) = $self.sys.$fn() {
            $labels.push(Label::new($val, n))
        }
    };
}

macro_rules! gauge_kb {
    ($metric:ident.$fn:ident = $name:expr) => {
        $metric.gauge($name, ($metric.sys.$fn() * 1024) as f64);
    };
}

macro_rules! gauge {
    ($metric:ident.$fn:ident = $name:expr) => {
        $metric.gauge($name, $metric.sys.$fn() as f64);
    };
}

impl Metric {
    /// Update gauge.
    pub fn gauge<K: Into<Key>>(&self, k: K, val: f64) {
        self.recorder
            .update_gauge(&k.into(), GaugeValue::Absolute(val));
    }

    /// Increment count.
    pub fn count_inc<K: Into<Key>>(&self, k: K, val: u64) {
        self.recorder.increment_counter(&k.into(), val);
    }

    /// Add listen state.
    pub fn add_listen_state(
        &self,
        listen: impl Fn(&Self) -> Result<(), PropertyError> + Send + 'static,
    ) {
        let mut guard = self.code.lock();
        guard.push(Box::new(listen));
    }

    fn flush(&self) -> Result<(), PropertyError> {
        let guard = self.code.lock();
        for i in guard.iter() {
            (i)(self)?;
        }
        Ok(())
    }

    /// Render metrics to prometheus format.
    pub fn render(&self) -> Result<String, PropertyError> {
        self.flush()?;
        Ok(self.handle.render())
    }

    fn register_sysinfo(&self) {
        let mut labels = vec![];
        set_val!(labels:self.get_name => "name");
        set_val!(labels:self.get_host_name => "hostname");
        set_val!(labels:self.get_kernel_version => "kernel_version");
        set_val!(labels:self.get_os_version => "os_version");
        set_val!(labels:self.get_long_os_version => "long_os_version");
        let key = Key::from_parts("uptime", labels);
        self.gauge(
            key,
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Not possible")
                .as_millis() as f64,
        );
        gauge_kb!(self.get_total_memory = "system.memory_total");
        gauge_kb!(self.get_total_swap = "system.memory_swap_total");
        self.add_listen_state(|metric| {
            // Memory
            gauge_kb!(metric.get_used_memory = "system.memory_used");
            gauge_kb!(metric.get_free_memory = "system.memory_free");
            gauge_kb!(metric.get_available_memory = "system.memory_available");
            gauge_kb!(metric.get_used_swap = "system.memory_swap_used");
            gauge_kb!(metric.get_free_swap = "system.memory_swap_free");
            // Component
            Ok(())
        });
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

        let recorder = builder.build();
        let handle = recorder.handle();

        let x = Metric {
            recorder,
            code: Mutex::new(Vec::new()),
            handle,
            sys: System::new_all(),
        };
        x.register_sysinfo();
        Ok(x)
    }

    fn order() -> Ordered {
        PRIORITY_HIGH
    }

    fn register_dependent_resources(builder: &mut FactoryBuilder<'_>) -> Result<(), PropertyError> {
        builder.submit(|req: Arc<Metric>| loop {
            #[cfg(feature = "log")]
            log::info!("PROMETHEUS: \n{}", req.render()?);
            sleep(Duration::from_secs(5));
        })
    }
}
