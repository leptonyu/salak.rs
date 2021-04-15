use log::*;
use salak::*;
use salak_factory::*;

fn main() {
    let env = Salak::new()
        .with_default_args(auto_read_sys_args_param!())
        .build();
    let log = env.build::<TracingLogConfig>().unwrap();

    let _guard = tracing::subscriber::set_default(log);

    for i in 0..10_000_000 {
        info!("Hello {} 0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000 {:0<10}", "world!", i);
    }
}
