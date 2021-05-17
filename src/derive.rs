use crate::*;

#[doc(hidden)]
pub trait AutoDeriveFromEnvironment: FromEnvironment {}

impl<P: AutoDeriveFromEnvironment> AutoDeriveFromEnvironment for Option<P> {}

#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
/// This trait is automatically derived, which is required by [`Environment::get()`].
pub trait PrefixedFromEnvironment: AutoDeriveFromEnvironment {
    /// Set configuration prefix.
    fn prefix() -> &'static str;
}

impl<P: PrefixedFromEnvironment> PrefixedFromEnvironment for Option<P> {
    fn prefix() -> &'static str {
        P::prefix()
    }
}

/// Key Description
#[derive(Debug)]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
pub struct KeyDesc {
    key: String,
    pub(crate) required: Option<bool>,
    def: Option<String>,
    pub(crate) desc: Option<String>,
    pub(crate) ignore: bool,
}

#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
impl std::fmt::Display for KeyDesc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{}\t{}\t{}\t{}",
            self.key,
            self.required.unwrap_or(true),
            self.def.as_ref().map(|f| f.as_ref()).unwrap_or(""),
            self.desc.as_ref().map(|f| f.as_ref()).unwrap_or("")
        ))
    }
}

#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
impl KeyDesc {
    pub(crate) fn new(
        key: String,
        required: Option<bool>,
        def: Option<&str>,
        desc: Option<String>,
    ) -> Self {
        Self {
            key,
            required,
            def: def.map(|c| c.to_string()),
            desc,
            ignore: true,
        }
    }

    pub(crate) fn set_required(&mut self, required: bool) {
        if self.required.is_none() {
            self.required = Some(required);
        }
    }
}

#[cfg(test)]
mod tests {

    use std::collections::HashMap;

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
        #[salak(desc = "must at least have one")]
        brr: NonEmptyVec<u8>,
        #[salak(desc = "map desc")]
        map: HashMap<String, u8>,
    }

    #[test]
    fn config_test() {
        let env = Salak::builder().set("salak.brr[0]", "1").unwrap_build();

        let config = env.get::<Config>().unwrap();

        assert_eq!("world", config.hello);
        assert_eq!(None, config.world);
        assert_eq!(None, config.hey);
        assert_eq!(123, config.num);
        let arr: Vec<u8> = vec![];
        assert_eq!(arr, config.arr);
        assert_eq!(vec![1], config.brr.0);

        println!("{:?}", config);

        for desc in env.get_desc::<Config>() {
            println!("{}", desc);
        }
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
