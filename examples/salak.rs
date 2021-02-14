use salak::environment::*;
use salak::Environment;

fn main() {
    let env = SourceRegistry::default();

    match env.require::<String>("hello") {
        Ok(val) => println!("{}", val),
        Err(e) => println!("{}", e),
    }
}
