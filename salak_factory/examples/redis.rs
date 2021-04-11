use salak_factory::*;

fn main() {
    for (k, o, v) in RedisConfig::list_keys("primary") {
        if let Some(v) = v {
            println!("{}[required={}]: {}", k, o, v);
        } else {
            println!("{}[required={}]", k, o);
        }
    }
}
