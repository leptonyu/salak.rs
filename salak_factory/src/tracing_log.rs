use ::tracing_log::LogTracer;
use chrono::{SecondsFormat, Utc};
use log::LevelFilter;
use std::sync::Arc;
use std::{cell::RefCell, io::Write, ops::RangeBounds, sync::Mutex};
use std::{fmt::Debug, io::BufWriter};
use tracing::{
    field::{Field, Visit},
    Event, Level, Subscriber,
};
use tracing_subscriber::{layer::Context, Layer};

use super::*;

/// Tracing log configuration
///
/// |property|required|default|
/// |-|-|-|
/// |logging.ignores|false||
/// |logging.max_level|false||
/// |logging.app_name|false|${app.name:}|
/// |logging.write_capacity|false|65536|
#[derive(FromEnvironment, Debug)]
#[salak(prefix = "logging")]
pub struct TracingLogConfig {
    ignores: Vec<String>,
    max_level: Option<LevelFilter>,
    #[salak(default = "${app.name:}")]
    app_name: Option<String>,
    #[salak(default = 8912)]
    buffer_size: usize,
}

/// Tracing Log customizer.
#[allow(missing_debug_implementations)]
pub struct TracingLogCustomizer {
    writer: Option<Box<dyn Write + 'static + Send>>,
}

impl Default for TracingLogCustomizer {
    fn default() -> Self {
        TracingLogCustomizer { writer: None }
    }
}

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct TracingLogWriter {
    name: Option<String>,
    writer: Arc<Mutex<BufWriter<Box<dyn Write + 'static + Send>>>>,
}

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct TracingLogGuard {
    writer: Arc<Mutex<BufWriter<Box<dyn Write + Send>>>>,
}

impl Drop for TracingLogGuard {
    fn drop(&mut self) {
        if let Ok(mut f) = self.writer.lock() {
            let _ = (*f).flush();
        }
    }
}

impl Buildable for TracingLogConfig {
    type Product = (TracingLogGuard, TracingLogWriter);

    type Customizer = TracingLogCustomizer;

    fn prefix() -> &'static str {
        "logging"
    }

    fn build_with_key(
        self,
        _: &impl Environment,
        customizer: Self::Customizer,
    ) -> Result<Self::Product, PropertyError> {
        let mut builder = LogTracer::builder();
        for ignore in self.ignores {
            builder = builder.ignore_crate(ignore);
        }
        if let Some(level) = self.max_level {
            builder = builder.with_max_level(level);
        }
        let w = if let Some(v) = customizer.writer {
            v
        } else {
            Box::new(std::io::stdout())
        };
        builder
            .init()
            .map_err(|e| PropertyError::ParseFail(format!("{}", e)))?;

        let wr = TracingLogWriter {
            name: self.app_name,
            writer: Arc::new(Mutex::new(BufWriter::with_capacity(self.buffer_size, w))),
        };
        Ok((
            TracingLogGuard {
                writer: wr.writer.clone(),
            },
            wr,
        ))
    }
}

struct EventWriter<'a>(&'a mut String);

impl Visit for EventWriter<'_> {
    #[inline]
    fn record_str(&mut self, f: &Field, value: &str) {
        if "message" == f.name() {
            self.0.push_str(value);
        }
    }

    #[inline]
    fn record_debug(&mut self, f: &Field, value: &dyn Debug) {
        if "message" == f.name() {
            use std::fmt::Write;
            let _ = write!(self.0, "{:?}", value);
        }
    }
}

struct LogBuf {
    buf: String,
    seconds: i64,
    milli: u32,
    time: (usize, usize, usize),
    lev: Level,
    level: (usize, usize),
    reserve: usize,
}

#[inline]
fn level_to_string(level: &Level) -> &str {
    match *level {
        Level::TRACE => "TRACE",
        Level::DEBUG => "DEBUG",
        Level::INFO => " INFO",
        Level::WARN => " WARN",
        Level::ERROR => "ERROR",
    }
}

impl LogBuf {
    fn new(name: &Option<String>, lev: &Level) -> Self {
        let mut buf = String::with_capacity(8192);
        let mut reserve = 0;
        let last = Utc::now();
        let time_str = last.to_rfc3339_opts(SecondsFormat::Millis, true);
        let len = time_str.len();
        reserve += len;
        let time = (0, len - 4, len - 1);
        buf.push_str(&time_str);
        reserve += 7;
        buf.push_str(&format!(" {} ", level_to_string(lev)));
        let level = (reserve - 6, reserve - 1);
        if let Some(name) = name {
            reserve += name.len() + 3;
            buf.push_str(&format!("[{}] ", name));
        }
        Self {
            buf,
            time,
            seconds: last.timestamp(),
            milli: last.timestamp_subsec_millis(),
            lev: lev.clone(),
            level,
            reserve,
        }
    }

    #[inline]
    fn reset(&mut self, level: &Level) {
        let now = Utc::now();
        let seconds = now.timestamp();
        if seconds != self.seconds {
            self.seconds = seconds;
            self.set_str(
                self.time.0..=self.time.2,
                &now.to_rfc3339_opts(SecondsFormat::Millis, true),
            );
        } else {
            let milli = now.timestamp_subsec_millis();
            if milli != self.milli {
                self.set_str(
                    self.time.1..self.time.2,
                    &format!("{:0>3}", now.timestamp_subsec_millis()),
                );
                self.milli = milli;
            }
        }

        if self.lev != *level {
            self.set_str(self.level.0..self.level.1, level_to_string(level));
        }

        self.buf.truncate(self.reserve);
    }

    #[inline]
    fn set_str<R: RangeBounds<usize>>(&mut self, range: R, msg: &str) {
        self.buf.replace_range(range, msg);
    }
}

impl TracingLogWriter {
    #[inline]
    fn write_log(&self, buf: &mut String, event: &Event<'_>) {
        if let Some(path) = event.metadata().module_path() {
            buf.push_str(path);
            buf.push(' ');
        }
        event.record(&mut EventWriter(buf));
        buf.push('\n');
        if let Ok(mut w) = self.writer.lock() {
            let _ = w.write_all(buf.as_bytes());
        }
    }
}

impl<S: Subscriber> Layer<S> for TracingLogWriter {
    #[inline]
    fn on_event(&self, event: &Event<'_>, _: Context<'_, S>) {
        thread_local! {
            static BUF: RefCell<Option<LogBuf>> = RefCell::new(None);
        }
        if event.metadata().name() != "log event" {
            return;
        }
        BUF.with(|buf| {
            if let Ok(mut opt_buf) = buf.try_borrow_mut() {
                if let Some(buf) = &mut *opt_buf {
                    buf.reset(event.metadata().level());
                    self.write_log(&mut buf.buf, event);
                } else {
                    let mut buf = LogBuf::new(&self.name, event.metadata().level());
                    self.write_log(&mut buf.buf, event);
                    *opt_buf = Some(buf);
                }
            }
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
