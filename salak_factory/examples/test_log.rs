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

    let mut join = vec![];
    let max = 100_1000;
    for i in 0..10 {
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
