use ::tracing_log::LogTracer;
use chrono::{DateTime, SecondsFormat, Utc};
use log::LevelFilter;
use ringbuf::*;
use std::fmt::Debug;
use std::sync::Arc;
use std::{
    cell::RefCell,
    io::{stdout, Stdout, Write},
};
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
pub struct LogConfig {
    ignores: Vec<String>,
    max_level: Option<LevelFilter>,
    #[salak(default = "${app.name:}")]
    app_name: Option<String>,
    #[salak(default = 8912)]
    buffer_size: usize,
}

impl Buildable for LogConfig {
    type Product = LogWriter;

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
        builder
            .init()
            .map_err(|e| PropertyError::ParseFail(format!("{}", e)))?;

        Ok(LogWriter {
            write: Arc::new(stdout()),
            buffer_size: self.buffer_size,
            app_name: self.app_name,
        })
    }
}

trait UpdateField {
    fn load(&mut self) -> &str;
}

impl UpdateField for Level {
    fn load(&mut self) -> &str {
        match *self {
            Level::TRACE => "TRACE",
            Level::DEBUG => "DEBUG",
            Level::INFO => "INFO",
            Level::WARN => "WARN",
            Level::ERROR => "ERROR",
        }
    }
}

struct FieldBuf<K> {
    key: K,
    value: String,
}

impl UpdateField for FieldBuf<DateTime<Utc>> {
    fn load(&mut self) -> &str {
        let key = Utc::now();
        let seconds = key.timestamp();
        if seconds != self.key.timestamp() {
            self.value = key.to_rfc3339_opts(SecondsFormat::Millis, true);
        } else if key.timestamp_subsec_millis() != self.key.timestamp_subsec_millis() {
            let n = self.value.len();
            self.value.replace_range(
                n - 4..n - 1,
                &format!("{:0>3}", self.key.timestamp_subsec_millis()),
            );
        }
        &self.value
    }
}

impl FieldBuf<DateTime<Utc>> {
    fn new() -> Self {
        let key = Utc::now();
        let value = key.to_rfc3339_opts(SecondsFormat::Millis, true);
        Self { key, value }
    }
}

struct LogBuffer {
    time: FieldBuf<DateTime<Utc>>,
    level: Level,
    name: Option<String>,
    pro: Producer<u8>,
    con: Consumer<u8>,
    out: Arc<Stdout>,
    reserve: usize,
    msg: String,
}

impl LogBuffer {
    fn new(level: &Level, out: Arc<Stdout>, buffer_size: usize, mut name: Option<String>) -> Self {
        let rb = RingBuffer::new(buffer_size);
        let (pro, con) = rb.split();
        let mut reserve = 27;
        if let Some(n) = &mut name {
            *n = format!("[{}] ", n);
            reserve += n.len();
        }
        LogBuffer {
            time: FieldBuf::new(),
            level: level.clone(),
            name,
            out,
            pro,
            con,
            reserve,
            msg: String::new(),
        }
    }
}

impl Drop for LogBuffer {
    fn drop(&mut self) {
        let _ = self.flush_all();
    }
}

impl LogBuffer {
    fn write_all(&mut self, msg: &dyn Debug) -> std::io::Result<usize> {
        self.msg.clear();
        use std::fmt::Write;
        let _ = writeln!(self.msg, "{:?}", msg);
        let size = self.reserve + self.msg.len();
        if self.pro.remaining() < size {
            self.flush_all()?;
        }
        let mut size = self.msg.len() + 1;
        let buf = self.time.load().as_bytes();
        size += buf.len() + 1;
        let _ = self.pro.write_all(buf);
        self.pro.write(b" ")?;
        let buf = self.level.load().as_bytes();
        size += buf.len() + 1;
        let _ = self.pro.write_all(buf);
        let _ = self.pro.write(b" ");
        if let Some(n) = &self.name {
            let _ = self.pro.write_all(n.as_bytes());
            size += n.len();
        }
        let _ = self.pro.write_all(self.msg.as_bytes());
        Ok(size)
    }

    fn flush_all(&mut self) -> std::io::Result<()> {
        let mut w = self.out.lock();
        while !self.con.is_empty() {
            let _ = self.con.write_into(&mut w, None);
        }
        Ok(())
    }
}

struct EventWriter<'a>(&'a mut LogBuffer);

impl Visit for EventWriter<'_> {
    #[inline]
    fn record_debug(&mut self, f: &Field, value: &dyn Debug) {
        if "message" == f.name() {
            let _ = self.0.write_all(value);
        }
    }
}

#[allow(missing_debug_implementations)]
#[doc(hidden)]
pub struct LogWriter {
    write: Arc<Stdout>,
    buffer_size: usize,
    app_name: Option<String>,
}

impl<S: Subscriber> Layer<S> for LogWriter {
    #[inline]
    fn on_event(&self, event: &Event<'_>, _: Context<'_, S>) {
        thread_local! {
            static BUF: RefCell<Option<LogBuffer>> = RefCell::new(None);
        }
        if event.metadata().name() != "log event" {
            return;
        }
        BUF.with(|buf| {
            if let Ok(mut opt_buf) = buf.try_borrow_mut() {
                if let Some(buf) = &mut *opt_buf {
                    event.record(&mut EventWriter(buf));
                } else {
                    let mut buf = LogBuffer::new(
                        event.metadata().level(),
                        self.write.clone(),
                        self.buffer_size,
                        self.app_name.clone(),
                    );
                    event.record(&mut EventWriter(&mut buf));
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
        print_keys::<LogConfig>();
    }
}
