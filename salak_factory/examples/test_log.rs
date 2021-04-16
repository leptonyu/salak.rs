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
}

fn main() {
    let env = Salak::new()
        .with_default_args(auto_read_sys_args_param!())
        .build();
    let max = env.load_config::<Max>().unwrap();
    let layer = env.build::<LogConfig>().unwrap();

    let _ = set_global_default(registry().with(layer));
    // let _ = env_logger::init();

    let num = num_cpus::get_physical();
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
