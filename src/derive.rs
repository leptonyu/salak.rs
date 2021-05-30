use crate::*;
use pad::{Alignment, PadStr};

/// This trait is automatically derived.
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
pub trait AutoDeriveFromEnvironment: FromEnvironment {}

#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
impl<P: AutoDeriveFromEnvironment> AutoDeriveFromEnvironment for Option<P> {}

#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
/// Context for implementing description.
#[allow(missing_debug_implementations)]
pub struct SalakDescContext<'a> {
    pub(crate) key: &'a mut Key<'a>,
    pub(crate) descs: &'a mut Vec<KeyDesc>,
    pub(crate) current: KeyDesc,
}

#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
/// Generate description for this object.
pub trait DescFromEnvironment: FromEnvironment {
    /// Generate key description from [`SalakDescContext`].
    /// * `env` - Describable context.
    fn key_desc(env: &mut SalakDescContext<'_>);
}

#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
/// [`FromEnvironment`] with a configuration prefix, which is required by [`Environment::get()`].
/// Attribute `#[salak(prefix = "salak.app")]` will implement this trait.
pub trait PrefixedFromEnvironment: DescFromEnvironment {
    /// Set configuration prefix.
    fn prefix() -> &'static str;
}

#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
impl<P: PrefixedFromEnvironment> PrefixedFromEnvironment for Option<P> {
    #[inline]
    fn prefix() -> &'static str {
        P::prefix()
    }
}
/// Key Description
#[derive(Debug)]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
pub(crate) struct KeyDesc {
    key: String,
    tp: &'static str,
    pub(crate) required: Option<bool>,
    def: Option<String>,
    pub(crate) desc: Option<String>,
    pub(crate) ignore: bool,
}

pub(crate) struct KeyDescs(pub(crate) Vec<KeyDesc>);

#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
impl std::fmt::Display for KeyDescs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut l1 = 3;
        let mut l2 = 8;
        let mut l3 = 7;
        let mut l4 = 11;
        // let mut l5 = 0;
        for desc in self.0.iter() {
            l1 = l1.max(desc.key.len());
            // l5 = l5.max(desc.tp.len());
            l2 = l2.max(desc.required.map(|_| 5).unwrap_or(0));
            l3 = l3.max(desc.def.as_ref().map(|def| def.len()).unwrap_or(0));
            l4 = l4.max(desc.desc.as_ref().map(|d| d.len()).unwrap_or(0));
        }

        f.write_fmt(format_args!(
            " {} | {} | {} | {} \n",
            "Key".pad_to_width_with_alignment(l1, Alignment::Middle),
            // "Type".pad_to_width_with_alignment(l5, Alignment::Middle),
            "Required".pad_to_width_with_alignment(l2, Alignment::Middle),
            "Default".pad_to_width_with_alignment(l3, Alignment::Middle),
            "Description".pad_to_width_with_alignment(l4, Alignment::Middle)
        ))?;
        f.write_fmt(format_args!(
            "{}+{}+{}+{}\n",
            "-".repeat(l1 + 2),
            // "-".repeat(l5 + 2),
            "-".repeat(l2 + 2),
            "-".repeat(l3 + 2),
            "-".repeat(l4 + 2),
        ))?;

        for desc in self.0.iter() {
            f.write_fmt(format_args!(
                " {} | {} | {} | {} \n",
                desc.key.pad_to_width_with_alignment(l1, Alignment::Left),
                // desc.tp.pad_to_width_with_alignment(l5, Alignment::Middle),
                desc.required
                    .unwrap_or(true)
                    .to_string()
                    .pad_to_width_with_alignment(l2, Alignment::Middle),
                desc.def
                    .as_ref()
                    .map(|f| f.as_ref())
                    .unwrap_or("")
                    .pad_to_width_with_alignment(l3, Alignment::Left),
                desc.desc
                    .as_ref()
                    .map(|f| f.as_ref())
                    .unwrap_or("")
                    .pad_to_width_with_alignment(l4, Alignment::Left)
            ))?;
        }
        Ok(())
    }
}

#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
impl KeyDesc {
    pub(crate) fn new(
        key: String,
        tp: &'static str,
        required: Option<bool>,
        def: Option<&str>,
        desc: Option<String>,
    ) -> Self {
        Self {
            key,
            tp,
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

    use lazy_static::__Deref;

    use crate::wrapper::NonEmptyVec;
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
        let env = Salak::builder().set("salak.brr[0]", "1").build().unwrap();

        let config = env.get::<Config>().unwrap();

        assert_eq!("world", config.hello);
        assert_eq!(None, config.world);
        assert_eq!(None, config.hey);
        assert_eq!(123, config.num);
        let arr: Vec<u8> = vec![];
        assert_eq!(arr, config.arr);
        assert_eq!(&vec![1], config.brr.deref());

        println!("{:?}", config);

        for desc in env.get_desc::<Config>() {
            println!("{:?}", desc);
        }
    }

    #[derive(FromEnvironment, Debug)]
    enum Value {
        Hello,
        World,
    }

    #[test]
    fn enum_test() {
        let env = Salak::builder().set("hello", "world").build().unwrap();
        println!("{:?}", env.require::<Value>("hello"))
    }

    #[test]
    fn derive_fail_test() {
        let t = trybuild::TestCases::new();
        t.compile_fail("tests/fail/*.rs");
    }
}
