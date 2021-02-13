use salak::Environment;
use salak::SourceRegistry;

fn main() {
    let env = SourceRegistry::default();

    match env.require::<String>("hello") {
        Ok(val) => println!("{}", val),
        Err(e) => println!("{}", e),
    }
}
