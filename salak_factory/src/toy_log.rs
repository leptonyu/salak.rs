use ::tracing_log::LogTracer;
use chrono::{SecondsFormat, Utc};
use log::LevelFilter;
use rtrb::*;
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
    fn load(&mut self) -> (&[u8], bool);
}

impl UpdateField for Level {
    #[inline]
    fn load(&mut self) -> (&[u8], bool) {
        (
            match *self {
                Level::TRACE => "TRACE",
                Level::DEBUG => "DEBUG",
                Level::INFO => "INFO",
                Level::WARN => "WARN",
                Level::ERROR => "ERROR",
            }
            .as_bytes(),
            false,
        )
    }
}

struct FieldBuf<K> {
    key: K,
    value: String,
}

impl UpdateField for FieldBuf<(i64, u32)> {
    #[inline]
    fn load(&mut self) -> (&[u8], bool) {
        let key = Utc::now();
        let seconds = key.timestamp();
        let mi = key.timestamp_subsec_millis();
        let mut updated = false;
        if seconds != self.key.0 {
            self.value = key.to_rfc3339_opts(SecondsFormat::Millis, true);
            self.key = (seconds, mi);
            updated = true;
        } else if mi != self.key.1 {
            let n = self.value.len();
            self.value
                .replace_range(n - 4..n - 1, &format!("{:0>3}", mi));
            self.key.1 = mi;
            updated = true;
        }
        (&self.value.as_bytes(), updated)
    }
}

impl FieldBuf<(i64, u32)> {
    fn new() -> Self {
        let key = Utc::now();
        let value = key.to_rfc3339_opts(SecondsFormat::Millis, true);
        let key = (key.timestamp(), key.timestamp_subsec_millis());
        Self { key, value }
    }
}

struct LogBuffer {
    time: FieldBuf<(i64, u32)>,
    level: Level,
    name: Option<Vec<u8>>,
    pro: Producer<u8>,
    con: Consumer<u8>,
    out: Arc<Stdout>,
    reserve: usize,
    msg: String,
}

impl LogBuffer {
    fn new(level: &Level, out: Arc<Stdout>, buffer_size: usize, name: Option<String>) -> Self {
        let rb = RingBuffer::new(buffer_size);
        let (pro, con) = rb.split();
        let mut reserve = 27;
        let name = if let Some(n) = name {
            let mut x = Vec::with_capacity(n.len() + 2);
            let _ = write!(&mut x, "[{}]", n);
            reserve += x.len();
            Some(x)
        } else {
            None
        };
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
        let _ = self.flush();
    }
}

impl LogBuffer {
    #[inline]
    fn write_debug(&mut self, path: Option<&str>, msg: &dyn Debug) -> std::io::Result<usize> {
        self.msg.clear();
        use std::fmt::Write;
        let _ = writeln!(self.msg, "{:?}", msg);
        self.write_str(path, None)
    }

    fn write_str(&mut self, path: Option<&str>, msg: Option<&str>) -> std::io::Result<usize> {
        let msg = match msg {
            Some(v) => v,
            _ => &self.msg,
        };

        let size = self.reserve + msg.len();
        let (time, updated) = self.time.load();
        let (level, _) = self.level.load();

        let buf = &[
            Some(time),
            Some(level),
            self.name.as_ref().map(|a| a.as_slice()),
            path.map(|a| a.as_bytes()),
        ];

        let size = if updated || self.pro.slots() < size {
            let mut w = self.out.lock();
            Self::flush_all(&mut w, &mut self.con)?;
            Self::write_buf(&mut w, buf, msg)
        } else {
            Self::write_buf(&mut self.pro, buf, msg)
        };
        Ok(size)
    }

    #[inline]
    fn write_buf(w: &mut dyn Write, buf: &[Option<&[u8]>], msg: &str) -> usize {
        let mut size = msg.len() + 1;
        for b in buf {
            if let Some(i) = b {
                size += i.len() + 1;
                let _ = w.write_all(i);
                let _ = w.write(b" ");
            }
        }
        let _ = w.write_all(msg.as_bytes());
        size
    }

    #[inline]
    fn flush_all(w: &mut dyn Write, con: &mut Consumer<u8>) -> std::io::Result<()> {
        if let Ok(chunk) = con.read_chunk(con.slots()) {
            let (a, b) = chunk.as_slices();
            w.write_all(a)?;
            w.write_all(b)?;
            chunk.commit_all();
        }
        Ok(())
    }

    #[inline]
    fn flush(&mut self) -> std::io::Result<()> {
        Self::flush_all(&mut self.out.lock(), &mut self.con)
    }
}

struct EventWriter<'a>(&'a mut LogBuffer, Option<&'a str>);

impl Visit for EventWriter<'_> {
    #[inline]
    fn record_str(&mut self, f: &Field, value: &str) {
        if "message" == f.name() {
            let _ = self.0.write_str(self.1, Some(value));
        }
    }
    #[inline]
    fn record_debug(&mut self, f: &Field, value: &dyn Debug) {
        if "message" == f.name() {
            let _ = self.0.write_debug(self.1, value);
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
                    event.record(&mut EventWriter(buf, event.metadata().module_path()));
                } else {
                    let mut buf = LogBuffer::new(
                        event.metadata().level(),
                        self.write.clone(),
                        self.buffer_size,
                        self.app_name.clone(),
                    );
                    event.record(&mut EventWriter(&mut buf, event.metadata().module_path()));
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
