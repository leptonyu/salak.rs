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
    #[salak(prefix = "salak")]
    struct Config {
        #[salak(default = "world")]
        hello: String,
        world: Option<String>,
        #[salak(name = "hello")]
        hey: Option<String>,
        #[salak(default = 123)]
        num: u8,
        arr: Vec<u8>,
    }

    #[test]
    fn config_test() {
        let env = Salak::builder().set("salak.arr[0]", "1").unwrap_build();

        let config = env.get::<Config>().unwrap();

        assert_eq!("world", config.hello);
        assert_eq!(None, config.world);
        assert_eq!(None, config.hey);
        assert_eq!(123, config.num);
        assert_eq!(vec![1], config.arr);

        println!("{:?}", config);
    }

    #[derive(FromEnvironment, Debug)]
    enum Value {
        Hello,
        World,
    }

    #[test]
    fn enum_test() {
        let env = Salak::builder().set("hello", "world").unwrap_build();
        println!("{:?}", env.require::<Value>("hello"))
    }
}
