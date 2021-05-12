use log::*;
use salak::*;
use salak_factory::*;

fn main() {
    let env = PropertyRegistry::new()
        .with_default_args(auto_read_sys_args_param!())
        .build();
    let _ = env.build::<LogConfig>().unwrap();
    for (k, o, v) in RedisConfig::list_keys(DEFAULT_NAMESPACE) {
        if let Some(v) = v {
            info!("{}[required={}]: {}", k, o, v);
        } else {
            info!("{}[required={}]", k, o);
        }
    }
}
