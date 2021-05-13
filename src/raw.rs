use crate::{Property, PropertyError};

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
        validate(&env, "z");
    }

    #[derive(Debug)]
    struct Config {
        i8: i8,
    }

    impl FromEnvironment for Config {
        fn from_env(
            key: &str,
            _: Option<Property<'_>>,
            env: &impl Environment,
        ) -> Result<Self, PropertyError> {
            Ok(Config {
                i8: env.require(&format!("{}.i8", key))?,
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
}
