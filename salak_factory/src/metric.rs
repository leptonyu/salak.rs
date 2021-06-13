//! Metric with prometheus
use core::f64;
pub use metrics::*;
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle, PrometheusRecorder};
use parking_lot::Mutex;
use salak::*;
use std::{
    any::Any,
    collections::HashSet,
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
    sys: Mutex<System>,
    enabled: bool,
    networks: HashSet<String>,
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
    ($labels:ident:$sys:ident.$fn:ident => $val:expr) => {
        if let Some(n) = $sys.$fn() {
            $labels.push(Label::new($val, n))
        }
    };
}

macro_rules! gauge_kb {
    ($metric:ident.$sys:ident.$fn:ident = $name:expr) => {
        $metric.gauge($name, ($sys.$fn() * 1024) as f64);
    };
}

macro_rules! gauge {
    ($metric:ident.$sys:ident.$fn:ident = $name:expr) => {
        $metric.gauge($name, $sys.$fn() as f64);
    };
}

macro_rules! gauge_network {
    ($metric:ident.$sys:ident.$fn:ident = $name:expr, $x:expr) => {
        $metric.gauge(
            Key::from_parts($name, vec![Label::new("network", $x.to_owned())]),
            $sys.$fn() as f64,
        );
    };
}

impl Metric {
    /// Update gauge.
    pub fn gauge<K: Into<Key>>(&self, k: K, val: f64) {
        if !self.enabled {
            return;
        }
        self.recorder
            .update_gauge(&k.into(), GaugeValue::Absolute(val));
    }

    /// Increment count.
    pub fn count_inc<K: Into<Key>>(&self, k: K, val: u64) {
        if !self.enabled {
            return;
        }
        self.recorder.increment_counter(&k.into(), val);
    }

    /// Add listen state.
    pub fn add_listen_state(
        &self,
        listen: impl Fn(&Self) -> Result<(), PropertyError> + Send + 'static,
    ) {
        if !self.enabled {
            return;
        }
        let mut guard = self.code.lock();
        guard.push(Box::new(listen));
    }

    fn flush(&self) -> Result<(), PropertyError> {
        if !self.enabled {
            return Ok(());
        }
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
        let sys = self.sys.lock();
        let mut labels = vec![];
        set_val!(labels:sys.get_name => "name");
        set_val!(labels:sys.get_host_name => "hostname");
        set_val!(labels:sys.get_kernel_version => "kernel_version");
        set_val!(labels:sys.get_os_version => "os_version");
        let key = Key::from_parts("uptime", labels);
        self.gauge(
            key,
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Not possible")
                .as_millis() as f64,
        );
        gauge_kb!(self.sys.get_total_memory = "system.memory_total");
        gauge_kb!(self.sys.get_total_swap = "system.swap_total");
        let pid = get_current_pid().unwrap();
        self.add_listen_state(move |metric| {
            let mut sys = metric.sys.lock();
            sys.refresh_memory();
            // Memory
            gauge_kb!(metric.sys.get_used_memory = "system.memory_used");
            gauge_kb!(metric.sys.get_free_memory = "system.memory_free");
            gauge_kb!(metric.sys.get_available_memory = "system.memory_available");
            gauge_kb!(metric.sys.get_used_swap = "system.swap_used");
            gauge_kb!(metric.sys.get_free_swap = "system.swap_free");
            // Process
            sys.refresh_process(pid);
            if let Some(process) = sys.get_processes().get(&pid) {
                gauge_kb!(metric.process.memory = "process.memory");
                gauge_kb!(metric.process.virtual_memory = "process.memory_virtual");
                gauge!(metric.process.start_time = "process.uptime");
                gauge!(metric.process.cpu_usage = "process.cpu_usage");
                let disk = process.disk_usage();
                metric.gauge(
                    "process.disk.total_written_bytes",
                    disk.total_written_bytes as f64,
                );
                metric.gauge("process.disk.written_bytes", disk.written_bytes as f64);
                metric.gauge(
                    "process.disk.total_read_bytes",
                    disk.total_read_bytes as f64,
                );
                metric.gauge("process.disk.read_bytes", disk.read_bytes as f64);
            }
            // Network
            sys.refresh_networks();
            for (name, nt) in sys.get_networks() {
                if !metric.networks.is_empty() && !metric.networks.contains(name) {
                    continue;
                }
                gauge_network!(
                    metric.nt.get_total_packets_received = "network.received.packets",
                    name
                );
                gauge_network!(
                    metric.nt.get_total_errors_on_received = "network.received.errors",
                    name
                );
                gauge_network!(
                    metric.nt.get_total_received = "network.received.total",
                    name
                );

                gauge_network!(
                    metric.nt.get_total_packets_transmitted = "network.transmitted.packets",
                    name
                );
                gauge_network!(
                    metric.nt.get_total_transmitted = "network.transmitted.total",
                    name
                );
                gauge_network!(
                    metric.nt.get_total_errors_on_transmitted = "network.transmitted.errors",
                    name
                );
            }
            sys.refresh_system();
            let load = sys.get_load_average();
            metric.gauge("system.load1", load.one);
            metric.gauge("system.load5", load.five);
            metric.gauge("system.load15", load.fifteen);
            Ok(())
        });
    }
}

/// Metric configuration.
#[derive(FromEnvironment, Debug)]
#[salak(prefix = "metric")]
pub struct MetricConfig {
    #[salak(desc = "Metric address, default is :9000")]
    address: Option<SocketAddr>,
    #[salak(desc = "Network metrics")]
    networks: HashSet<String>,
    #[salak(default = "true")]
    enabled: bool,
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
            sys: Mutex::new(System::new_all()),
            enabled: config.enabled,
            networks: config.networks,
        };
        x.register_sysinfo();
        Ok(x)
    }

    fn order() -> Ordered {
        PRIORITY_HIGH
    }

    fn register_dependent_resources(builder: &mut FactoryBuilder<'_>) -> Result<(), PropertyError> {
        builder.submit(|_req: Arc<Metric>| loop {
            #[cfg(feature = "log")]
            log::info!("PROMETHEUS: \n{}", _req.render()?);
            sleep(Duration::from_secs(5));
        })
    }
}
