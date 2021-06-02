#[cfg(feature = "derive")]
use crate::{DescFromEnvironment, PrefixedFromEnvironment, SalakDescContext};
use crate::{FromEnvironment, PropertyError, SalakContext};
use std::{
    collections::HashSet,
    ffi::OsString,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
    path::PathBuf,
    time::Duration,
};

/// Raw property, it is a temprory representation of property, which can be either [`&str`] or [`String`], or other values.
#[derive(Clone, Debug)]
pub enum Property<'a> {
    /// [`&str`] holder.
    S(&'a str),
    /// [`String`] holder.
    O(String),
    /// Number holder.
    I(i64),
    /// Float holder.
    F(f64),
    /// Bool holder.
    B(bool),
}

/// Any object implements this trait is automatically implmenting [`crate::FromEnvironment`].
///
/// This trait defines how to parse value from property, and defines specific behaviors such as
/// how empty string being parsed.
pub trait IsProperty: Sized {
    /// Check if empty string means property does not exist.
    /// In most case this is true, except String.
    #[inline]
    fn is_empty(p: &Property<'_>) -> bool {
        match p {
            Property::S(s) => s.is_empty(),
            Property::O(s) => s.is_empty(),
            _ => false,
        }
    }

    /// Parse value from property.
    fn from_property(_: Property<'_>) -> Result<Self, PropertyError>;
}

impl<T: IsProperty> FromEnvironment for T {
    #[inline]
    fn from_env(
        val: Option<Property<'_>>,
        env: &mut SalakContext<'_>,
    ) -> Result<Self, PropertyError> {
        if let Some(v) = val {
            if !Self::is_empty(&v) {
                return Self::from_property(v);
            }
        }
        Err(PropertyError::NotFound(env.current_key().to_string()))
    }
}

#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
impl<T: IsProperty> DescFromEnvironment for T {
    #[inline]
    fn key_desc(env: &mut SalakDescContext<'_>) {
        env.current.ignore = false;
        env.current.set_required(true);
    }
}

impl IsProperty for () {
    fn from_property(_: Property<'_>) -> Result<Self, PropertyError> {
        Ok(())
    }
}

impl PrefixedFromEnvironment for () {
    fn prefix() -> &'static str {
        ""
    }
}

#[inline]
fn check_f64(f: f64) -> Result<f64, PropertyError> {
    if f.is_finite() {
        Ok(f)
    } else {
        Err(PropertyError::parse_fail("f64 value is infinite"))
    }
}

impl IsProperty for String {
    #[inline]
    fn is_empty(_: &Property<'_>) -> bool {
        false
    }
    #[inline]
    fn from_property(p: Property<'_>) -> Result<Self, PropertyError> {
        Ok(match p {
            Property::S(v) => v.to_string(),
            Property::O(v) => v,
            Property::I(v) => v.to_string(),
            Property::F(v) => check_f64(v)?.to_string(),
            Property::B(v) => v.to_string(),
        })
    }
}
impl IsProperty for bool {
    #[inline]
    fn from_property(p: Property<'_>) -> Result<Self, PropertyError> {
        fn str_to_bool(v: &str) -> Result<bool, PropertyError> {
            match v {
                "yes" | "true" => Ok(true),
                "no" | "false" => Ok(false),
                _ => Err(PropertyError::parse_fail("invalid bool value")),
            }
        }
        match p {
            Property::B(v) => Ok(v),
            Property::S(v) => str_to_bool(v),
            Property::O(v) => str_to_bool(&v),
            _ => Err(PropertyError::parse_fail("can not num to bool")),
        }
    }
}

macro_rules! impl_property_num {
    ($($x:ident),+) => {$(
            impl IsProperty for $x {
                #[inline]
                fn from_property(p: Property<'_>) -> Result<Self, PropertyError> {
                    use std::convert::TryFrom;
                    Ok(match p {
                    Property::S(s) => s.parse::<$x>()?,
                    Property::O(s) => s.parse::<$x>()?,
                    Property::I(s) => $x::try_from(s)?,
                    Property::F(s) => check_f64(s)? as $x,
                    _ => return Err(PropertyError::parse_fail("can not convert bool to num")),
                    })
                }

            }

            )+}
}

impl_property_num!(i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, isize, usize);

macro_rules! impl_property_float {
    ($($x:ident),+) => {$(
            #[allow(trivial_numeric_casts)]
            impl IsProperty for $x {
                #[inline]
                fn from_property(p: Property<'_>) -> Result<Self, PropertyError> {
                    Ok(match p {
                    Property::S(s) => s.parse::<$x>()?,
                    Property::O(s) => s.parse::<$x>()?,
                    Property::I(s) => s as $x,
                    Property::F(s) => check_f64(s)? as $x,
                    _ => return Err(PropertyError::parse_fail("can not convert bool to num")),
                    })
                }

            }

            )+}
}

impl_property_float!(f32, f64);

#[inline]
fn parse_duration_from_str(du: &str) -> Result<Duration, PropertyError> {
    let mut i = 0;
    let mut multi = 1;
    let mut last = None;
    for c in du.chars().rev() {
        match c {
            'h' | 'm' | 's' if last.is_none() => {
                if c == 'm' {
                    last = Some('M');
                } else {
                    last = Some(c);
                }
            }
            'm' | 'u' | 'n' if last == Some('s') => {
                last = Some(c);
            }
            c if ('0'..='9').contains(&c) => {
                if last.is_none() {
                    last = Some('s');
                }
                i += multi * (c as u64 - '0' as u64);
                multi *= 10;
            }
            _ => return Err(PropertyError::parse_fail("Invalid duration")),
        }
    }
    Ok(match last.unwrap_or('s') {
        'h' => Duration::new(i * 3600, 0),
        'M' => Duration::new(i * 60, 0),
        's' => Duration::from_secs(i),
        'm' => Duration::from_millis(i),
        'u' => Duration::from_micros(i),
        'n' => Duration::from_nanos(i),
        _ => return Err(PropertyError::parse_fail("Invalid duration")),
    })
}

impl IsProperty for Duration {
    fn from_property(p: Property<'_>) -> Result<Self, PropertyError> {
        match p {
            Property::O(du) => parse_duration_from_str(&du),
            Property::S(du) => parse_duration_from_str(du),
            Property::I(seconds) => Ok(Duration::from_secs(seconds as u64)),
            Property::F(sec) => Ok(Duration::new(0, 0).mul_f64(sec)),
            Property::B(_) => Err(PropertyError::parse_fail("bool cannot convert to duration")),
        }
    }
}

/// Sub key is partial [`Key`] having values with either `[a-z][_a-z0-9]*` or [`usize`].
#[derive(Debug)]
pub(crate) enum SubKey<'a> {
    /// Str sub key.
    S(&'a str),
    /// Index sub key.
    I(usize),
}

lazy_static::lazy_static! {
    static ref P: &'static [char] = &['.', '[', ']'];
}
/// Key with a string buffer, can be avoid allocating memory when parsing configuration.
#[derive(Debug)]
pub struct Key<'a> {
    buf: String,
    key: Vec<SubKey<'a>>,
}

impl<'a> Key<'a> {
    #[inline]
    pub(crate) fn new() -> Self {
        Self {
            buf: String::new(),
            key: vec![],
        }
    }

    pub(crate) fn from_str(key: &'a str) -> Self {
        let mut k = Self::new();
        for n in key.split(&P[..]) {
            if let Some(c) = n.chars().next() {
                if c.is_ascii_digit() {
                    if let Ok(v) = n.parse() {
                        k.push(SubKey::I(v));
                        continue;
                    }
                }
                k.push(SubKey::S(n));
            }
        }
        k
    }

    #[allow(dead_code)]
    pub(crate) fn as_generic(&self) -> String {
        self.as_str().replace("[0]", "[*]")
    }

    #[allow(dead_code)]
    pub(crate) fn iter(&self) -> std::slice::Iter<'_, SubKey<'_>> {
        self.key.iter()
    }

    pub(crate) fn as_str(&self) -> &str {
        if self.buf.starts_with('.') {
            return &self.buf.as_str()[1..];
        }
        self.buf.as_str()
    }

    pub(crate) fn push(&mut self, k: SubKey<'a>) {
        match &k {
            SubKey::S(v) => {
                self.buf.push('.');
                self.buf.push_str(*v);
            }
            SubKey::I(v) => {
                self.buf.push_str(&format!("[{}]", *v));
            }
        }
        self.key.push(k)
    }

    pub(crate) fn pop(&mut self) {
        if let Some(v) = self.key.pop() {
            match v {
                SubKey::S(n) => self.buf.truncate(self.buf.len() - n.len() - 1),
                SubKey::I(n) => self.buf.truncate(self.buf.len() - n.to_string().len() - 2),
            }
        }
    }
}

impl<'a> From<&'a str> for SubKey<'a> {
    fn from(mut u: &'a str) -> Self {
        if u.starts_with('[') {
            u = &u[1..];
            let mut x = 0;
            for i in u.chars() {
                if ('0'..='9').contains(&i) {
                    x = x * 10 + (i as usize) - ('0' as usize);
                } else {
                    break;
                }
            }
            return SubKey::I(x);
        }
        SubKey::S(u)
    }
}

impl From<usize> for SubKey<'_> {
    #[inline]
    fn from(u: usize) -> Self {
        SubKey::I(u)
    }
}
/// Sub key collection, which stands for lists of sub keys with same prefix.
#[derive(Debug)]
pub struct SubKeys<'a> {
    keys: HashSet<&'a str>,
    upper: Option<usize>,
}

impl<'a> SubKeys<'a> {
    /// Insert a sub key.
    pub(crate) fn insert<K: Into<SubKey<'a>>>(&mut self, key: K) {
        match key.into() {
            SubKey::S(s) => {
                self.keys.insert(s);
            }
            SubKey::I(i) => {
                if let Some(max) = self.upper {
                    if i <= max {
                        return;
                    }
                }
                self.upper = Some(i);
            }
        }
    }

    pub(crate) fn str_keys(&self) -> Vec<&'a str> {
        self.keys
            .iter()
            .filter(|a| {
                if let Some(c) = a.chars().next() {
                    c < '0' && c > '9'
                } else {
                    false
                }
            })
            .copied()
            .collect()
    }

    #[inline]
    pub(crate) fn new() -> Self {
        Self {
            keys: HashSet::new(),
            upper: None,
        }
    }

    #[inline]
    pub(crate) fn max(&self) -> Option<usize> {
        self.upper
    }
}

macro_rules! impl_property_from_str {
    ($($x:ident),+) => {$(
            impl IsProperty for $x {
                #[inline]
                fn from_property(p: Property<'_>) -> Result<Self, PropertyError> {
                    use std::str::FromStr;
                    Ok(match p {
                    Property::S(s) => <$x>::from_str(s)?,
                    Property::O(s) => <$x>::from_str(&s)?,
                    _ => return Err(PropertyError::parse_fail("can not convert")),
                    })
                }

            }
            )+}
}

impl_property_from_str!(
    Ipv4Addr,
    Ipv6Addr,
    IpAddr,
    SocketAddrV4,
    SocketAddrV6,
    SocketAddr,
    PathBuf,
    OsString
);

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn property_test() {
        let env = Salak::builder()
            .set("a", "0")
            .set("b", "${b}")
            .set("c", "${a}")
            .set("d", "${z}")
            .set("e", "${z:}")
            .set("f", "${z:${a}}")
            .set("g", "a")
            .set("h", "${${g}}")
            .set("i", "\\$\\{a\\}")
            .set("j", "${${g}:a}")
            .set("k", "${a} ${a}")
            .set("l", "${c}")
            .set("m", "${no_found:${no_found_2:hello}}")
            .build()
            .unwrap();

        fn validate<T: std::fmt::Debug + FromEnvironment>(env: &Salak, key: &str, val: &str) {
            println!("{} key: {}", std::any::type_name::<T>(), key);
            assert_eq!(val, &format!("{:?}", env.require::<T>(key)));
        }

        validate::<String>(&env, "a", "Ok(\"0\")");
        validate::<String>(&env, "b", "Err(RecursiveFail(\"b\"))");
        validate::<String>(&env, "c", "Ok(\"0\")");
        validate::<String>(&env, "d", "Err(ResolveNotFound(\"z\"))");
        validate::<String>(&env, "e", "Ok(\"\")");
        validate::<String>(&env, "f", "Ok(\"0\")");
        validate::<String>(&env, "g", "Ok(\"a\")");
        validate::<String>(&env, "h", "Ok(\"0\")");
        validate::<String>(&env, "i", "Ok(\"${a}\")");
        validate::<String>(&env, "j", "Ok(\"0\")");
        validate::<String>(&env, "k", "Ok(\"0 0\")");
        validate::<String>(&env, "l", "Ok(\"0\")");
        validate::<String>(&env, "m", "Ok(\"hello\")");

        validate::<bool>(
            &env,
            "a",
            "Err(ParseFail(None, SalakParseError(\"invalid bool value\")))",
        );
        validate::<bool>(&env, "b", "Err(RecursiveFail(\"b\"))");
        validate::<bool>(
            &env,
            "c",
            "Err(ParseFail(None, SalakParseError(\"invalid bool value\")))",
        );
        validate::<bool>(&env, "d", "Err(ResolveNotFound(\"z\"))");
        validate::<bool>(&env, "e", "Err(NotFound(\"e\"))");
        validate::<bool>(
            &env,
            "f",
            "Err(ParseFail(None, SalakParseError(\"invalid bool value\")))",
        );
        validate::<bool>(
            &env,
            "g",
            "Err(ParseFail(None, SalakParseError(\"invalid bool value\")))",
        );
        validate::<bool>(
            &env,
            "h",
            "Err(ParseFail(None, SalakParseError(\"invalid bool value\")))",
        );
        validate::<bool>(
            &env,
            "i",
            "Err(ParseFail(None, SalakParseError(\"invalid bool value\")))",
        );
        validate::<bool>(
            &env,
            "j",
            "Err(ParseFail(None, SalakParseError(\"invalid bool value\")))",
        );
        validate::<bool>(
            &env,
            "k",
            "Err(ParseFail(None, SalakParseError(\"invalid bool value\")))",
        );
        validate::<bool>(
            &env,
            "l",
            "Err(ParseFail(None, SalakParseError(\"invalid bool value\")))",
        );
        validate::<bool>(
            &env,
            "m",
            "Err(ParseFail(None, SalakParseError(\"invalid bool value\")))",
        );

        validate::<u8>(&env, "a", "Ok(0)");
        validate::<u8>(&env, "b", "Err(RecursiveFail(\"b\"))");
        validate::<u8>(&env, "c", "Ok(0)");
        validate::<u8>(&env, "d", "Err(ResolveNotFound(\"z\"))");
        validate::<u8>(&env, "e", "Err(NotFound(\"e\"))");
        validate::<u8>(&env, "f", "Ok(0)");
        validate::<u8>(
            &env,
            "g",
            "Err(ParseFail(None, ParseIntError { kind: InvalidDigit }))",
        );
        validate::<u8>(&env, "h", "Ok(0)");
        validate::<u8>(
            &env,
            "i",
            "Err(ParseFail(None, ParseIntError { kind: InvalidDigit }))",
        );
        validate::<u8>(&env, "j", "Ok(0)");
        validate::<u8>(
            &env,
            "k",
            "Err(ParseFail(None, ParseIntError { kind: InvalidDigit }))",
        );
        validate::<u8>(&env, "l", "Ok(0)");
        validate::<u8>(
            &env,
            "m",
            "Err(ParseFail(None, ParseIntError { kind: InvalidDigit }))",
        );

        validate::<Option<u8>>(&env, "a", "Ok(Some(0))");
        validate::<Option<u8>>(&env, "b", "Err(RecursiveFail(\"b\"))");
        validate::<Option<u8>>(&env, "c", "Ok(Some(0))");
        validate::<Option<u8>>(&env, "d", "Err(ResolveNotFound(\"z\"))");
        validate::<Option<u8>>(&env, "e", "Ok(None)");
        validate::<Option<u8>>(&env, "f", "Ok(Some(0))");
        validate::<Option<u8>>(
            &env,
            "g",
            "Err(ParseFail(None, ParseIntError { kind: InvalidDigit }))",
        );
        validate::<Option<u8>>(&env, "h", "Ok(Some(0))");
        validate::<Option<u8>>(
            &env,
            "i",
            "Err(ParseFail(None, ParseIntError { kind: InvalidDigit }))",
        );
        validate::<Option<u8>>(&env, "j", "Ok(Some(0))");
        validate::<Option<u8>>(
            &env,
            "k",
            "Err(ParseFail(None, ParseIntError { kind: InvalidDigit }))",
        );
        validate::<Option<u8>>(&env, "l", "Ok(Some(0))");
        validate::<Option<u8>>(
            &env,
            "m",
            "Err(ParseFail(None, ParseIntError { kind: InvalidDigit }))",
        );
    }

    #[test]
    fn bool_tests() {
        assert_eq!(true, bool::from_property(Property::S("yes")).unwrap());
        assert_eq!(true, bool::from_property(Property::S("true")).unwrap());
        assert_eq!(false, bool::from_property(Property::S("no")).unwrap());
        assert_eq!(false, bool::from_property(Property::S("false")).unwrap());

        assert_eq!(true, bool::from_property(Property::S("x")).is_err());
        assert_eq!(true, bool::from_property(Property::S("n")).is_err());
        assert_eq!(true, bool::from_property(Property::S("f")).is_err());
        assert_eq!(true, bool::from_property(Property::S("y")).is_err());
        assert_eq!(true, bool::from_property(Property::S("t")).is_err());
        assert_eq!(true, bool::from_property(Property::I(0)).is_err());
        assert_eq!(true, bool::from_property(Property::I(1)).is_err());
        assert_eq!(true, bool::from_property(Property::F(0.0)).is_err());
        assert_eq!(true, bool::from_property(Property::F(1.0)).is_err());
    }

    #[quickcheck]
    fn num_tests(i: i64) {
        assert_eq!(
            i,
            i64::from_property(Property::O(format!("{}", i))).unwrap()
        );
        assert_eq!(i, i64::from_property(Property::I(i)).unwrap());
        assert_eq!(true, i64::from_property(Property::B(true)).is_err());
    }

    #[quickcheck]
    fn i64_convert_tests(i: i64) -> bool {
        let u8: Result<u8, PropertyError> = IsProperty::from_property(Property::I(i));
        let u16: Result<u16, PropertyError> = IsProperty::from_property(Property::I(i));
        let u32: Result<u32, PropertyError> = IsProperty::from_property(Property::I(i));
        let u64: Result<u64, PropertyError> = IsProperty::from_property(Property::I(i));
        let u128: Result<u128, PropertyError> = IsProperty::from_property(Property::I(i));
        let i8: Result<i8, PropertyError> = IsProperty::from_property(Property::I(i));
        let i16: Result<i16, PropertyError> = IsProperty::from_property(Property::I(i));
        let i32: Result<i32, PropertyError> = IsProperty::from_property(Property::I(i));
        let i64: Result<i64, PropertyError> = IsProperty::from_property(Property::I(i));
        let i128: Result<i128, PropertyError> = IsProperty::from_property(Property::I(i));
        let f32: Result<f32, PropertyError> = IsProperty::from_property(Property::I(i));
        let f64: Result<f64, PropertyError> = IsProperty::from_property(Property::I(i));
        vec![
            i >= 0 && i <= (u8::MAX as i64) && u8.is_ok() || u8.is_err(),
            i >= 0 && i <= (u16::MAX as i64) && u16.is_ok() || u16.is_err(),
            i >= 0 && i <= (u32::MAX as i64) && u32.is_ok() || u32.is_err(),
            i >= 0 && u64.is_ok() || u64.is_err(),
            i >= 0 && u128.is_ok() || u128.is_err(),
            i >= (i8::MIN as i64) && i <= (i8::MAX as i64) && i8.is_ok() || i8.is_err(),
            i >= (i16::MIN as i64) && i <= (i16::MAX as i64) && i16.is_ok() || i16.is_err(),
            i >= (i32::MIN as i64) && i <= (i32::MAX as i64) && i32.is_ok() || i32.is_err(),
            i64.is_ok(),
            i128.is_ok(),
            f32.is_ok() && f32.unwrap_or(0.0).is_finite(),
            f64.is_ok() && f64.unwrap_or(0.0).is_finite(),
        ]
        .iter()
        .all(|a| *a)
    }

    #[quickcheck]
    fn f64_convert_tests(i: f64) -> bool {
        let u8: Result<u8, PropertyError> = IsProperty::from_property(Property::F(i));
        let u16: Result<u16, PropertyError> = IsProperty::from_property(Property::F(i));
        let u32: Result<u32, PropertyError> = IsProperty::from_property(Property::F(i));
        let u64: Result<u64, PropertyError> = IsProperty::from_property(Property::F(i));
        let u128: Result<u128, PropertyError> = IsProperty::from_property(Property::F(i));
        let i8: Result<i8, PropertyError> = IsProperty::from_property(Property::F(i));
        let i16: Result<i16, PropertyError> = IsProperty::from_property(Property::F(i));
        let i32: Result<i32, PropertyError> = IsProperty::from_property(Property::F(i));
        let i64: Result<i64, PropertyError> = IsProperty::from_property(Property::F(i));
        let i128: Result<i128, PropertyError> = IsProperty::from_property(Property::F(i));
        let f32: Result<f32, PropertyError> = IsProperty::from_property(Property::F(i));
        let f64: Result<f64, PropertyError> = IsProperty::from_property(Property::F(i));

        vec![
            i.is_finite() && u8.is_ok() || u8.is_err(),
            i.is_finite() && u16.is_ok() || u16.is_err(),
            i.is_finite() && u32.is_ok() || u32.is_err(),
            i.is_finite() && u64.is_ok() || u64.is_err(),
            i.is_finite() && u128.is_ok() || u128.is_err(),
            i.is_finite() && i8.is_ok() || i8.is_err(),
            i.is_finite() && i16.is_ok() || i16.is_err(),
            i.is_finite() && i32.is_ok() || i32.is_err(),
            i.is_finite() && i64.is_ok() || i64.is_err(),
            i.is_finite() && i128.is_ok() || i128.is_err(),
            i.is_finite() && f32.is_ok() || f32.is_err(),
            i.is_finite() && f64.is_ok() || f64.is_err(),
        ]
        .iter()
        .all(|a| *a)
    }

    #[test]
    fn duration_test() {
        use super::*;
        assert_eq!(
            Duration::new(123, 0),
            parse_duration_from_str("123").unwrap()
        );
        assert_eq!(
            Duration::new(123, 0),
            parse_duration_from_str("123s").unwrap()
        );
        assert_eq!(
            Duration::new(10 * 60, 0),
            parse_duration_from_str("10m").unwrap()
        );
        assert_eq!(
            Duration::new(123 * 3600, 0),
            parse_duration_from_str("123h").unwrap()
        );
        assert_eq!(
            Duration::new(0, 123 * 1000_000),
            parse_duration_from_str("123ms").unwrap()
        );
        assert_eq!(
            Duration::new(0, 123 * 1000),
            parse_duration_from_str("123us").unwrap()
        );
        assert_eq!(
            Duration::new(0, 123),
            parse_duration_from_str("123ns").unwrap()
        );
        assert_eq!(
            Duration::new(1, 0),
            parse_duration_from_str("1000ms").unwrap()
        );
    }

    #[derive(Debug)]
    struct Config {
        i8: i8,
    }

    impl FromEnvironment for Config {
        fn from_env(
            _: Option<Property<'_>>,
            env: &mut SalakContext<'_>,
        ) -> Result<Self, PropertyError> {
            Ok(Config {
                i8: env.require_def("i8", None)?,
            })
        }
    }

    #[cfg(feature = "derive")]
    #[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
    impl DescFromEnvironment for Config {
        fn key_desc(env: &mut SalakDescContext<'_>) {
            env.add_key_desc::<i8>("i8", None, None, None);
        }
    }
    #[test]
    fn config_test() {
        let env = Salak::builder()
            .set("a", "0")
            .set("b", "${b}")
            .set("c", "${a}")
            .set("d", "${z}")
            .set("e", "${z:}")
            .set("f", "${z:${a}}")
            .set("g", "a")
            .set("h", "${${g}}")
            .set("i", "\\$\\{a\\}")
            .set("j", "${${g}:a}")
            .set("k", "${a} ${a}")
            .set("l", "${c}")
            .build()
            .unwrap();
        println!("{:?}", env.require::<Config>(""));
        println!("{:?}", env.require::<Option<Config>>(""));
    }

    #[test]
    fn key_test() {
        fn assert_key(prefix: &str, target: &str) {
            assert_eq!(Key::from_str(prefix).as_str(), target);
        }

        assert_key("salak.prop", "salak.prop");
        assert_key(".salak.prop", "salak.prop");
        assert_key("[]salak.prop", "salak.prop");
        assert_key("[0]salak.prop", "[0].salak.prop");
        assert_key("salak[0].prop", "salak[0].prop");
        assert_key("salak.0.prop", "salak[0].prop");
        assert_key("", "");
        assert_key("hello", "hello");
        assert_key(".", "");
        assert_key("[0]", "[0]");
        assert_key("0", "[0]");
    }

    #[test]
    fn key_modification_test() {
        fn assert_key<'a>(key: &mut Key<'a>, target: &'a str) {
            let prefix = key.as_str().to_string();
            let p = key.as_str().to_string();
            key.push(SubKey::S(target));
            assert_eq!(key.as_str(), &format!("{}.{}", p, target));
            key.pop();
            assert_eq!(prefix, key.as_str());
        }

        fn assert_keys(key: &str, targets: Vec<&str>) {
            let mut key = Key::from_str(key);
            for target in targets {
                assert_key(&mut key, target);
            }
        }

        assert_keys("redis", vec!["port", "host", "ssl", "pool"]);
        assert_keys("hello.hey", vec!["world"]);
        assert_keys("hello[0].hey", vec!["world"]);
    }
}
