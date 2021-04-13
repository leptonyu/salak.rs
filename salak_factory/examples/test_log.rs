use log::*;
use salak::*;
use salak_factory::*;

fn main() {
    let env = Salak::new()
        .with_default_args(auto_read_sys_args_param!())
        .build();
    let _guard = env.build::<TracingLogConfig>().unwrap();

    for i in 0..100 {
        info!("Hello {:0>100}", i);
    }
}
