use log::*;
use salak::*;
use salak_factory::*;
use tracing::subscriber::set_global_default;
use tracing_subscriber::prelude::*;
use tracing_subscriber::registry;

fn main() {
    let env = Salak::new()
        .with_default_args(auto_read_sys_args_param!())
        .build();
    let (_guard, layer) = env.build::<TracingLogConfig>().unwrap();

    let _ = set_global_default(registry().with(layer));

    let max = 1000_000;
    let num = num_cpus::get();
    let mut join = vec![];
    for i in 0..num {
        join.push(std::thread::spawn(move || {
            let max = max / 10;
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
