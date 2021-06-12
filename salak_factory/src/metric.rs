//! Metric with prometheus
use core::f64;
use ipnet::IpNet;
use metrics::*;
use metrics_exporter_prometheus::PrometheusBuilder;
use salak::*;
use std::{net::SocketAddr, time::UNIX_EPOCH};

/// Metric.
#[allow(missing_debug_implementations, missing_copy_implementations)]
pub struct Metric;

/// Metric configuration.
#[derive(FromEnvironment, Debug)]
#[salak(prefix = "metric")]
pub struct MetricConfig {
    #[salak(default = "${salak.app.name}")]
    application: String,
    #[salak(default = "${salak.app.version}")]
    version: String,
    address: Option<SocketAddr>,
    allowed: Vec<IpNet>,
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
        for allow in config.allowed {
            builder = builder.add_allowed(allow);
        }
        builder.install()?;
        gauge!(
            "uptime",
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)?
                .as_millis() as f64
        );
        Ok(Metric)
    }

    fn order() ->Ordered{
      PRIORITY_HIGH
    }
}
