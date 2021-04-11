use ::tracing_log::LogTracer;
use chrono::{SecondsFormat, Utc};
use log::LevelFilter;
use std::cell::RefCell;
use std::fmt::Debug;
use std::fmt::Write;
use tracing::{
    field::{Field, Visit},
    subscriber::set_global_default,
    Event, Subscriber,
};
use tracing_subscriber::{
    layer::{Context, SubscriberExt},
    registry::Registry,
    Layer,
};

use super::*;

/// Tracing log configuration
///
/// |property|required|default|
/// |-|-|-|
/// |logging.ignores|false||
/// |logging.max_level|false||
#[derive(FromEnvironment, Debug)]
pub struct TracingLogConfig {
    ignores: Vec<String>,
    max_level: Option<LevelFilter>,
    subscribe: TracingLogSubscriberConfig,
}

#[derive(FromEnvironment, Debug, Clone)]
struct TracingLogSubscriberConfig {
    #[salak(default = "${app.name}")]
    app_name: String,
}

impl Buildable for TracingLogConfig {
    type Product = ();

    type Customizer = ();

    fn prefix() -> &'static str {
        "logging"
    }

    fn build_with_key(
        self,
        _: &impl Environment,
        _: Self::Customizer,
    ) -> Result<Self::Product, PropertyError> {
        let mut builder = LogTracer::builder();
        for ignore in self.ignores {
            builder = builder.ignore_crate(ignore);
        }
        if let Some(level) = self.max_level {
            builder = builder.with_max_level(level);
        }
        let registry = Registry::default().with(self.subscribe.clone());
        set_global_default(registry).map_err(|e| PropertyError::ParseFail(format!("{}", e)))?;

        builder
            .init()
            .map_err(|e| PropertyError::ParseFail(format!("{}", e)))
    }
}

struct EventWriter<'a>(&'a mut String);

impl Visit for EventWriter<'_> {
    fn record_debug(&mut self, f: &Field, value: &dyn Debug) {
        if let "message" = f.name() {
            let _ = write!(self.0, "{:?}", value);
        }
    }
}

impl<S: Subscriber> Layer<S> for TracingLogSubscriberConfig {
    fn on_event(&self, event: &Event<'_>, _: Context<'_, S>) {
        thread_local! {
            static BUF: RefCell<String> = RefCell::new(String::new());
        }

        BUF.with(|buf| {
            let borrow = buf.try_borrow_mut();
            let mut a;
            let mut b;
            let mut buf = match borrow {
                Ok(buf) => {
                    a = buf;
                    &mut *a
                }
                _ => {
                    b = String::new();
                    &mut b
                }
            };
            let _ = write!(
                &mut buf,
                "{} {}",
                Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
                event.metadata().level()
            );
            let _ = write!(&mut buf, " [{}]", self.app_name);
            let _ = write!(
                &mut buf,
                " {}: ",
                event.metadata().module_path().unwrap_or("")
            );
            event.record(&mut EventWriter(&mut buf));
            let _ = writeln!(&mut buf);
            use std::io::Write;
            let _ = std::io::stdout().write_all(buf.as_bytes());
            buf.clear();
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tracing_log_tests() {
        print_keys::<TracingLogConfig>();
    }
}
