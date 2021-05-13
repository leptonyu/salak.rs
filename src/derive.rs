use crate::*;

#[doc(hidden)]
pub trait AutoDeriveFromEnvironment: FromEnvironment {}

impl<P: AutoDeriveFromEnvironment> AutoDeriveFromEnvironment for Option<P> {}

#[doc(hidden)]
pub trait DefaultSourceFromEnvironment: AutoDeriveFromEnvironment {
    fn prefix() -> &'static str;
}

impl<P: DefaultSourceFromEnvironment> DefaultSourceFromEnvironment for Option<P> {
    fn prefix() -> &'static str {
        P::prefix()
    }
}

#[cfg(test)]
mod tests {

    use crate::*;

    #[derive(FromEnvironment, Debug)]
    struct Config {
        #[salak(default = "world")]
        hello: String,
        world: Option<String>,
        #[salak(name = "hello")]
        hey: Option<String>,
    }

    #[test]
    fn config_test() {
        let env = Salak::new().unwrap();

        println!("{:?}", env.require::<Config>("hello"))
    }
}
