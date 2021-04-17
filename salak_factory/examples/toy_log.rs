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
    let max = env.load_config::<Max>().unwrap();
    if max.env_log {
        let _ = env_logger::init();
    } else {
        let _ = set_global_default(registry().with(env.build::<LogConfig>().unwrap()));
    }

    let num = max.thread.unwrap_or(num_cpus::get_physical()).max(1);
    let max = max.count / num;
    let mut join = vec![];
    for i in 0..num {
        join.push(std::thread::spawn(move || {
            let i = i * max;
            for j in 0..max {
                info!("Hello {:0>10}", i + j);
            }
        }));
    }

    for h in join {
        let _ = h.join();
    }
}
