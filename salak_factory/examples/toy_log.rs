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
    #[salak(default = false)]
    env_log: bool,
}

fn main() {
    let env = Salak::new()
        .with_default_args(auto_read_sys_args_param!())
        .build();
    let conf = env.load_config::<Max>().unwrap();
    if conf.env_log {
        let _ = env_logger::builder()
            .filter_level(LevelFilter::Info)
            .target(env_logger::Target::Stdout)
            .init();
    } else {
        let _ = set_global_default(registry().with(env.build::<LogConfig>().unwrap()));
    }

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
        "Record {} logs in {}ms, {}ns/log, {}/s, {}/s/thread",
        total,
        time / 1000_000,
        time / (total as i64),
        ((num * total) as i64) * 1000_000_000 / time,
        (total as i64) * 1000_000_000 / time
    );
}
