use salak::Environment;
use salak::Salak;

fn main() {
    env_logger::init();

    let env = Salak::default();

    match env.require::<String>("a.b.c.hello") {
        Ok(val) => println!("{}", val),
        Err(e) => println!("{}", e),
    }
}
