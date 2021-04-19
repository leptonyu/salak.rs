use std::any::Any;

use chrono::Utc;
use log::*;
use salak::*;
use salak_factory::*;
use tracing::subscriber::set_global_default;
use tracing_subscriber::prelude::*;
use tracing_subscriber::registry;

#[derive(FromEnvironment, Debug)]
#[salak(prefix = "")]
struct Max {
    #[salak(default = 10_000_000)]
    count: usize,
    thread: Option<usize>,
    #[salak(default = toy)]
    logger: String,
    #[salak(default = 0)]
    mode: u8,
}

fn main() {
    let env = Salak::new()
        .with_default_args(auto_read_sys_args_param!())
        .build();
    let conf = env.load_config::<Max>().unwrap();

    let _any: Box<dyn Any> = match &conf.logger[..] {
        "fern" => Box::new(init_fern()),
        "env" => Box::new(init_env()),
        "log4rs" => Box::new(init_log4rs()),
        "slog" => init_slog(),
        "fast" => Box::new(init_fast()),
        "toy" => Box::new(init_toy(&env)),
        _ => panic!("No specified"),
    };

    if conf.mode == 0 {
        let num = conf.thread.unwrap_or(num_cpus::get_physical()).max(1);
        let total = conf.count;
        let max = conf.count / num;
        let mut join = vec![];
        for i in 0..num {
            join.push(std::thread::spawn(move || {
                let t = Utc::now();
                let i = i * max;
                for j in 0..max {
                    info!("Hello {:0>10}", i + j);
                }
                Utc::now().timestamp_nanos() - t.timestamp_nanos()
            }));
        }

        let mut time = 0;
        for h in join {
            if let Ok(t) = h.join() {
                time += t;
            }
        }
        eprintln!(
            "{}: Record {} logs in {}ms, {}ns/log, {}/s, {}/s/thread",
            conf.logger,
            total,
            time / 1000_000,
            time / (total as i64),
            ((num * total) as i64) * 1000_000_000 / time,
            (total as i64) * 1000_000_000 / time
        );
    } else if conf.mode == 1 {
        for i in 0..10 {
            info!("{} Hello", i);
            info!("{} World", i);
            std::thread::sleep(std::time::Duration::from_secs(3));
        }
    }
}

fn init_fern() {
    fern::Dispatch::new()
        // Perform allocation-free log formatting
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        // Add blanket level filter -
        .level(log::LevelFilter::Debug)
        // Output to stdout, files, and other Dispatch configurations
        .chain(std::io::stdout())
        // Apply globally
        .apply()
        .unwrap();
}

fn init_log4rs() {
    use log4rs::append::console::ConsoleAppender;
    use log4rs::config::{Appender, Config, Root};
    let stdout = ConsoleAppender::builder().build();

    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .build(Root::builder().appender("stdout").build(LevelFilter::Info))
        .unwrap();

    log4rs::init_config(config).unwrap();
}

fn init_env() {
    let _ = env_logger::builder()
        .filter_level(LevelFilter::Info)
        .target(env_logger::Target::Stdout)
        .write_style(env_logger::WriteStyle::Never)
        .init();
}

fn init_toy(env: &Salak) {
    let logger = env.build::<LogConfig>().unwrap();
    let _ = set_global_default(registry().with(logger));
}

fn init_slog() -> Box<dyn Any> {
    use slog::*;
    let plain = slog_term::PlainSyncDecorator::new(std::io::stdout());
    let logger = Logger::root(slog_term::FullFormat::new(plain).build().fuse(), o!());
    let guard = slog_scope::set_global_logger(logger);
    slog_stdlog::init().unwrap();
    Box::new(guard)
}

fn init_fast() {
    use fast_log::plugin::console::ConsoleAppender;
    let _ = fast_log::fast_log::init_custom_log(
        vec![Box::new(ConsoleAppender {})],
        1000,
        Level::Info,
        Box::new(fast_log::filter::NoFilter {}),
        Box::new(fast_log::appender::FastLogFormatRecord {}),
    );
}
