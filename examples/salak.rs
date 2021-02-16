use salak::*;

fn main() {
    env_logger::init();

    let env = SalakBuilder::new()
        .with_default_args(auto_read_sys_args_param!())
        .build();

    match env.require::<String>("hello") {
        Ok(val) => println!("{}", val),
        Err(e) => println!("{}", e),
    }
}
