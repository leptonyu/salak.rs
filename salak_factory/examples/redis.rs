use log::*;
use salak::*;
use salak_factory::*;

fn main() {
    let env = Salak::new()
        .with_default_args(auto_read_sys_args_param!())
        .build();
    let _ = env.build::<TracingLogConfig>().unwrap();
    for (k, o, v) in RedisConfig::list_keys("primary") {
        if let Some(v) = v {
            info!("{}[required={}]: {}", k, o, v);
        } else {
            info!("{}[required={}]", k, o);
        }
    }
}
