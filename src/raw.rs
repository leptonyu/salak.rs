use crate::PropertyError;
use std::collections::HashSet;

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

/// This trait defines how to parse value from property, and defines specific behaviors such as
/// how empty string being parsed.
pub trait IsProperty: Sized {
    /// Check if empty string means property does not exist.
    /// In most case this is true, except String.
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

fn check_f64(f: f64) -> Result<f64, PropertyError> {
    if f.is_finite() {
        Ok(f)
    } else {
        Err(PropertyError::parse_fail("f64 value is infinite"))
    }
}

impl IsProperty for String {
    fn is_empty(_: &Property<'_>) -> bool {
        false
    }
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

        fn validate(env: &Salak, key: &str) {
            println!("{}: {:?}", key, env.require::<String>(key));
            println!("{}: {:?}", key, env.require::<bool>(key));
            println!("{}: {:?}", key, env.require::<u8>(key));
            println!("{}: {:?}", key, env.require::<Option<u8>>(key));
        }

        validate(&env, "a");
        validate(&env, "b");
        validate(&env, "c");
        validate(&env, "d");
        validate(&env, "e");
        validate(&env, "f");
        validate(&env, "g");
        validate(&env, "h");
        validate(&env, "i");
        validate(&env, "j");
        validate(&env, "k");
        validate(&env, "l");
        validate(&env, "m");
        validate(&env, "z");
    }

    #[derive(Debug)]
    struct Config {
        i8: i8,
    }

    impl FromEnvironment for Config {
        fn from_env<'a>(
            key: &mut Key<'a>,
            _: Option<Property<'_>>,
            env: &'a impl Environment,
        ) -> Result<Self, PropertyError> {
            Ok(Config {
                i8: env.require_def(key, SubKey::S("i8"), None)?,
            })
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
}

/// Sub key is partial [`Key`] having values with either `[_a-zA-Z0-9]+` or [`usize`].
#[derive(Debug)]
pub enum SubKey<'a> {
    /// Str sub key.
    S(&'a str),
    /// Index sub key.
    I(usize),
}

/// Key has a string buffer, used for avoid allocate memory when parsing properties.
#[derive(Debug)]
pub struct Key<'a> {
    buf: String,
    key: Vec<SubKey<'a>>,
}

lazy_static::lazy_static! {
    static ref P: &'static [char] = &['.', '[', ']'];
}

impl<'a> Key<'a> {
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

    pub(crate) fn iter(&self) -> std::slice::Iter<'_, SubKey<'_>> {
        self.key.iter()
    }

    pub(crate) fn as_str(&self) -> &str {
        if self.buf.chars().next() == Some('.') {
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

/// Sub key collection, which stands for lists of sub keys with same prefix.
#[derive(Debug)]
pub struct SubKeys<'a> {
    pub(crate) keys: HashSet<&'a str>,
    pub(crate) upper: Option<usize>,
}

impl<'a> Into<SubKey<'a>> for &'a str {
    fn into(self) -> SubKey<'a> {
        SubKey::S(self)
    }
}
impl<'a> Into<SubKey<'a>> for usize {
    fn into(self) -> SubKey<'a> {
        SubKey::I(self)
    }
}

impl<'a> SubKeys<'a> {
    /// Insert a sub key.
    pub fn insert<K: Into<SubKey<'a>>>(&mut self, key: K) {
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
}
