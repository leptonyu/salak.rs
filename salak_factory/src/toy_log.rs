use ::tracing_log::LogTracer;
use chrono::{SecondsFormat, Utc};
use log::{LevelFilter, Log, Metadata, Record};
use rtrb::*;
use std::{
    cell::RefCell,
    fmt::{Arguments, Debug},
    io::{stdout, ErrorKind, Stdout, Write},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, Weak,
    },
    thread::JoinHandle,
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
/// |logging.buffer_size|false|8912|
/// |logging.enable_tracing|false|false|
#[cfg_attr(docsrs, doc(cfg(feature = "enable_log")))]
#[derive(FromEnvironment, Debug)]
#[salak(prefix = "logging")]
pub struct LogConfig {
    ignores: Vec<String>,
    max_level: Option<LevelFilter>,
    #[salak(default = "${app.name:}")]
    app_name: Option<String>,
    #[salak(default = 8912)]
    buffer_size: usize,
    #[salak(default = false)]
    enable_tracing: bool,
}

impl Buildable for LogConfig {
    type Product = Option<LogWriter>;

    type Customizer = ();

    fn prefix() -> &'static str {
        "logging"
    }

    fn build_with_key(
        self,
        _: &impl Environment,
        _: Self::Customizer,
    ) -> Result<Self::Product, PropertyError> {
        let rb: RingBuffer<LogBufferFlush> = RingBuffer::new(1024);
        let (pro, mut con) = rb.split();
        let _flush_thread: JoinHandle<()> = std::thread::Builder::new()
            .name("logger_flush".to_owned())
            .spawn(move || {
                let mut lbf = vec![];
                loop {
                    while let Ok(v) = con.pop() {
                        lbf.push(v);
                    }
                    for v in lbf.iter() {
                        if let Ok(ab) = v.dirty.lock() {
                            if let Ok(true) = ab.compare_exchange(
                                true,
                                false,
                                Ordering::Acquire,
                                Ordering::Relaxed,
                            ) {
                                v.flush();
                            }
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
            })?;
        let log = LogWriter {
            write: Arc::new(stdout()),
            queue: Mutex::new(pro),
            buffer_size: self.buffer_size,
            app_name: self.app_name,
            max_level: self.max_level.unwrap_or(LevelFilter::Info),
            _flush_thread,
        };
        if self.enable_tracing {
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
            Ok(Some(log))
        } else {
            log::set_max_level(log.max_level.clone());
            let _ = log::set_boxed_logger(Box::new(log));
            Ok(None)
        }
    }
}

trait UpdateField {
    fn load(&mut self) -> (&[u8], bool);
}

impl UpdateField for &log::Level {
    #[inline]
    fn load(&mut self) -> (&[u8], bool) {
        (
            match **self {
                log::Level::Trace => "TRACE",
                log::Level::Debug => "DEBUG",
                log::Level::Info => "INFO",
                log::Level::Warn => "WARN",
                log::Level::Error => "ERROR",
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
    name: Option<Vec<u8>>,
    pro: Producer<u8>,
    con: Arc<Mutex<Consumer<u8>>>,
    out: Arc<Stdout>,
    size: usize,
    msg: String,
    dirty: Arc<Mutex<AtomicBool>>,
}

struct LogBufferFlush {
    con: Weak<Mutex<Consumer<u8>>>,
    out: Arc<Stdout>,
    dirty: Arc<Mutex<AtomicBool>>,
}

impl LogBufferFlush {
    fn flush(&self) {
        if let Some(con) = self.con.upgrade() {
            let _ = LogBuffer::flush_all(&mut self.out.lock(), &con);
        }
    }
}

impl LogBuffer {
    fn get_flush(&self) -> LogBufferFlush {
        LogBufferFlush {
            con: Arc::downgrade(&self.con),
            out: self.out.clone(),
            dirty: self.dirty.clone(),
        }
    }

    fn new(out: Arc<Stdout>, buffer_size: usize, name: Option<String>) -> Self {
        let rb = RingBuffer::new(buffer_size);
        let (pro, con) = rb.split();
        let time = FieldBuf::new();
        let mut size = time.value.len() + 1;
        let name = if let Some(n) = name {
            let mut x = Vec::with_capacity(n.len() + 2);
            let _ = write!(&mut x, "[{}]", n);
            size += x.len() + 1;
            Some(x)
        } else {
            None
        };
        LogBuffer {
            time,
            name,
            out,
            pro,
            con: Arc::new(Mutex::new(con)),
            size,
            msg: String::new(),
            dirty: Arc::new(Mutex::new(AtomicBool::new(false))),
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
    fn write_debug(
        &mut self,
        level: &log::Level,
        path: Option<&str>,
        msg: &dyn Debug,
    ) -> std::io::Result<usize> {
        self.msg.clear();
        use std::fmt::Write;
        let _ = writeln!(self.msg, "{:?}", msg);
        self.write_str(level, path, None)
    }

    fn write_args(
        &mut self,
        level: &log::Level,
        path: Option<&str>,
        msg: &Arguments<'_>,
    ) -> std::io::Result<usize> {
        self.msg.clear();
        use std::fmt::Write;
        let _ = writeln!(self.msg, "{:?}", msg);
        self.write_str(level, path, None)
    }

    fn write_str(
        &mut self,
        mut level: &log::Level,
        path: Option<&str>,
        msg: Option<&[u8]>,
    ) -> std::io::Result<usize> {
        let msg = match msg {
            Some(v) => v,
            _ => &self.msg.as_bytes(),
        };

        let (time, updated) = self.time.load();
        let (level, _) = level.load();

        let buf = &[
            Some(time),
            Some(level),
            self.name.as_ref().map(|a| a.as_slice()),
            path.map(|a| a.as_bytes()),
        ];

        let mut size = msg.len() + self.size + 1;
        if let Some(p) = path {
            size += p.len() + 1;
        }

        if updated || self.pro.slots() < size {
            let mut w = self.out.lock();
            Self::flush_all(&mut w, &self.con)?;
            Self::write_buf(&mut w, buf, msg)?;
        } else {
            Self::write_buf(&mut self.pro, buf, msg)?;
            self.set_dirty();
        };
        Ok(size)
    }

    fn set_dirty(&self) {
        if let Ok(mut guard) = self.dirty.lock() {
            *guard.get_mut() = true;
        }
    }

    #[inline]
    fn write_buf(
        w: &mut dyn Write,
        buf: &[Option<&[u8]>],
        msg: &[u8],
    ) -> Result<(), std::io::Error> {
        for b in buf {
            if let Some(i) = b {
                w.write_all(*i)?;
                w.write(b" ")?;
            }
        }
        w.write_all(msg)?;
        Ok(())
    }

    #[inline]
    fn flush_all(w: &mut dyn Write, con: &Arc<Mutex<Consumer<u8>>>) -> std::io::Result<()> {
        if let Ok(mut con) = con.lock() {
            let size = con.slots();
            if let Ok(chunk) = con.read_chunk(size) {
                let (a, b) = chunk.as_slices();
                w.write_all(a)?;
                w.write_all(b)?;
                chunk.commit_all();
            }
        }
        Ok(())
    }

    #[inline]
    fn flush(&mut self) -> std::io::Result<()> {
        Self::flush_all(&mut self.out.lock(), &self.con)
    }
}

struct EventWriter<'a>(
    &'a mut LogBuffer,
    &'a log::Level,
    Option<&'a str>,
    std::io::Result<usize>,
);

impl Visit for EventWriter<'_> {
    #[inline]
    fn record_str(&mut self, f: &Field, value: &str) {
        if "message" == f.name() {
            self.3 = self.0.write_str(self.1, self.2, Some(value.as_bytes()));
        }
    }
    #[inline]
    fn record_debug(&mut self, f: &Field, value: &dyn Debug) {
        if "message" == f.name() {
            self.3 = self.0.write_debug(self.1, self.2, value);
        }
    }
}

/// Log writer.
#[allow(missing_debug_implementations)]
#[cfg_attr(docsrs, doc(cfg(feature = "enable_log")))]
pub struct LogWriter {
    write: Arc<Stdout>,
    queue: Mutex<Producer<LogBufferFlush>>,
    buffer_size: usize,
    app_name: Option<String>,
    max_level: LevelFilter,
    _flush_thread: JoinHandle<()>,
}

thread_local! {
    static BUF: RefCell<Option<LogBuffer>> = RefCell::new(None);
}

impl LogWriter {
    #[inline]
    fn with_buf<F: FnMut(&mut LogBuffer) -> std::io::Result<usize>>(
        &self,
        mut f: F,
    ) -> std::io::Result<usize> {
        BUF.with(|buf| {
            if let Ok(mut opt_buf) = buf.try_borrow_mut() {
                if let Some(buf) = &mut *opt_buf {
                    return (f)(buf);
                } else {
                    let mut buf =
                        LogBuffer::new(self.write.clone(), self.buffer_size, self.app_name.clone());
                    if let Ok(mut q) = self.queue.lock() {
                        let _ = q.push(buf.get_flush());
                    }
                    let size = (f)(&mut buf);
                    *opt_buf = Some(buf);
                    return size;
                }
            }
            Err(ErrorKind::WouldBlock.into())
        })
    }
}

impl<S: Subscriber> Layer<S> for LogWriter {
    #[inline]
    fn on_event(&self, event: &Event<'_>, _: Context<'_, S>) {
        if event.metadata().name() != "log event" {
            return;
        }
        let _ = self.with_buf(|buf| {
            let level = convert(event.metadata().level());
            let mut x = EventWriter(buf, &level, event.metadata().module_path(), Ok(0));
            event.record(&mut x);
            x.3
        });
    }
}

impl Log for LogWriter {
    fn enabled(&self, md: &Metadata<'_>) -> bool {
        self.max_level >= md.level()
    }

    fn log(&self, record: &Record<'_>) {
        let _ =
            self.with_buf(|lb| lb.write_args(&record.level(), record.module_path(), record.args()));
    }

    fn flush(&self) {
        let _ = self.write.lock().flush();
    }
}

#[inline]
fn convert(level: &Level) -> log::Level {
    match *level {
        Level::TRACE => log::Level::Trace,
        Level::DEBUG => log::Level::Debug,
        Level::INFO => log::Level::Info,
        Level::WARN => log::Level::Warn,
        Level::ERROR => log::Level::Error,
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
