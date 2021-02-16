use salak::*;

fn main() {
    env_logger::init();

    let env = SalakBuilder::new()
        .with_args_param(sys_args_param!())
        .build();

    match env.require::<String>("a.b.c.hello") {
        Ok(val) => println!("{}", val),
        Err(e) => println!("{}", e),
    }
}
