//! [`Property`] converter.
use crate::*;
use core::convert::TryFrom;

/// Convert to [`Property`].
pub trait ToProperty: Sized {
    fn to_property(self) -> Property;
}

impl ToProperty for Property {
    fn to_property(self) -> Property {
        self
    }
}

impl ToProperty for String {
    fn to_property(self) -> Property {
        Property::Str(self)
    }
}

impl ToProperty for &str {
    fn to_property(self) -> Property {
        Property::Str(self.to_owned())
    }
}

macro_rules! impl_to_property {
    ($x:ident) => {
        impl ToProperty for $x {
            fn to_property(self) -> Property {
                Property::Int(self as i64)
            }
        }
    };
    ($x:ident, $($y:ident),+) => {
        impl_to_property!($x);
        impl_to_property!($($y),+);
    };
}

impl_to_property!(u8, u16, u32, i8, i16, i32, i64);

macro_rules! impl_to_property_str {
    ($x:ident) => {
        impl ToProperty for $x {
            fn to_property(self) -> Property {
                Property::Str(self.to_string())
            }
        }
    };
    ($x:ident, $($y:ident),+) => {
        impl_to_property_str!($x);
        impl_to_property_str!($($y),+);
    };
}

impl_to_property_str!(u64, u128, i128);

macro_rules! impl_float_to_property {
    ($x:ident) => {
        impl ToProperty for $x {
            fn to_property(self) -> Property {
                Property::Float(self as f64)
            }
        }
    };
    ($x:ident, $($y:ident),+) => {
        impl_float_to_property!($x);
        impl_float_to_property!($($y),+);
    };
}

impl_float_to_property!(f32, f64);

/// Convert value from [`Property`].
pub trait FromProperty: Sized {
    fn from_property(_: Property) -> Result<Self, PropertyError>;
}

impl FromProperty for Property {
    fn from_property(a: Property) -> Result<Self, PropertyError> {
        Ok(a)
    }
}

fn check_f64(f: f64) -> Result<f64, PropertyError> {
    if f.is_finite() {
        Ok(f)
    } else {
        Err(PropertyError::ParseFail("f64 value is infinite".to_owned()))
    }
}

impl FromProperty for String {
    fn from_property(p: Property) -> Result<Self, PropertyError> {
        match p {
            Property::Str(str) => Ok(str),
            Property::Int(i64) => Ok(i64.to_string()),
            Property::Float(f64) => Ok(check_f64(f64)?.to_string()),
            Property::Bool(bool) => Ok(bool.to_string()),
        }
    }
}

macro_rules! impl_from_property {
    ($x:ident) => {
        impl FromProperty for $x {
            fn from_property(p: Property) -> Result<Self, PropertyError> {
                match p {
                    Property::Str(str) => str
                        .parse::<$x>()
                        .map_err(|e| PropertyError::ParseFail(e.to_string())),
                    Property::Int(i64) => <$x>::try_from(i64).map_err(|e|PropertyError::ParseFail(e.to_string())),
                    Property::Float(f64) => Ok(check_f64(f64)? as $x),
                    Property::Bool(_) => Err(PropertyError::ParseFail(
                        format!("Bool value cannot convert to {}",stringify!($x)),
                    )),
                }
            }
        }
    };
    ($x:ident, $($y:ident),+) => {
        impl_from_property!($x);
        impl_from_property!($($y),+);
    };
}

impl_from_property!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);

macro_rules! impl_float_from_property {
    ($x:ident) => {
        impl FromProperty for $x {
            fn from_property(p: Property) -> Result<Self, PropertyError> {
                match p {
                    Property::Str(str) => str
                        .parse::<$x>()
                        .map_err(|e| PropertyError::ParseFail(e.to_string())),
                    Property::Int(i64) => Ok(i64 as $x),
                    Property::Float(f64) => Ok(check_f64(f64)? as $x),
                    Property::Bool(_) => Err(PropertyError::ParseFail(
                        format!("Bool value cannot convert to {}",stringify!($x)),
                    )),
                }
            }
        }
    };
    ($x:ident, $($y:ident),+) => {
        impl_float_from_property!($x);
        impl_float_from_property!($($y),+);
    };
}

impl_float_from_property!(f64, f32);

impl FromProperty for bool {
    fn from_property(p: Property) -> Result<Self, PropertyError> {
        lazy_static::lazy_static! {
        static ref STR_YES: HashSet<String> = vec!["yes","y","1","true","t"].into_iter().map(|a|format!("{}",a)).collect();
        static ref STR_NO: HashSet<String> = vec!["no","n","0","false","f"].into_iter().map(|a|format!("{}",a)).collect();
        }
        match p {
            Property::Str(str) => {
                let str = str.to_lowercase();
                if STR_YES.contains(&str) {
                    Ok(true)
                } else if STR_NO.contains(&str) {
                    Ok(false)
                } else {
                    Err(PropertyError::ParseFail(format!(
                        "Str cannot convert to bool"
                    )))
                }
            }
            Property::Int(i64) => Ok(i64 != 0),
            Property::Float(f64) => Ok(check_f64(f64)? != 0.0),
            Property::Bool(bool) => Ok(bool),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    #[test]
    fn bool_tests() {
        assert_eq!(
            Ok(true),
            bool::from_property(Property::Str("yes".to_owned()))
        );
        assert_eq!(Ok(true), bool::from_property(Property::Str("y".to_owned())));
        assert_eq!(Ok(true), bool::from_property(Property::Str("1".to_owned())));
        assert_eq!(
            Ok(true),
            bool::from_property(Property::Str("true".to_owned()))
        );
        assert_eq!(Ok(true), bool::from_property(Property::Str("t".to_owned())));
        assert_eq!(Ok(true), bool::from_property(Property::Int(1)));
        assert_eq!(Ok(true), bool::from_property(Property::Float(1.0)));
        assert_eq!(Ok(true), bool::from_property(Property::Bool(true)));

        assert_eq!(
            Ok(false),
            bool::from_property(Property::Str("no".to_owned()))
        );
        assert_eq!(
            Ok(false),
            bool::from_property(Property::Str("n".to_owned()))
        );
        assert_eq!(
            Ok(false),
            bool::from_property(Property::Str("0".to_owned()))
        );
        assert_eq!(
            Ok(false),
            bool::from_property(Property::Str("false".to_owned()))
        );
        assert_eq!(
            Ok(false),
            bool::from_property(Property::Str("f".to_owned()))
        );
        assert_eq!(Ok(false), bool::from_property(Property::Int(0)));
        assert_eq!(Ok(false), bool::from_property(Property::Float(0.0)));
        assert_eq!(Ok(false), bool::from_property(Property::Bool(false)));

        assert_eq!(
            true,
            bool::from_property(Property::Str("x".to_owned())).is_err()
        );
    }

    #[test]
    fn option_test() {
        assert_eq!(
            Ok(None),
            <Option<String>>::from_err(PropertyError::NotFound("".to_owned()))
        );
    }

    #[quickcheck]
    fn num_tests(i: i64) {
        assert_eq!(Ok(i), i64::from_property(Property::Str(format!("{}", i))));
        assert_eq!(Ok(i), i64::from_property(Property::Int(i as i64)));
        assert_eq!(
            Err(PropertyError::ParseFail(
                "Bool value cannot convert to i64".to_owned()
            )),
            i64::from_property(Property::Bool(true))
        );
    }

    #[quickcheck]
    fn u8_tests(i: u8) -> bool {
        FromProperty::from_property(ToProperty::to_property(i)) == Ok(i)
    }

    #[quickcheck]
    fn u16_tests(i: u16) -> bool {
        FromProperty::from_property(ToProperty::to_property(i)) == Ok(i)
    }

    #[quickcheck]
    fn u32_tests(i: u32) -> bool {
        FromProperty::from_property(ToProperty::to_property(i)) == Ok(i)
    }

    #[quickcheck]
    fn u64_tests(i: u64) -> bool {
        FromProperty::from_property(ToProperty::to_property(i)) == Ok(i)
    }

    #[quickcheck]
    fn u128_tests(i: u128) -> bool {
        FromProperty::from_property(ToProperty::to_property(i)) == Ok(i)
    }

    #[quickcheck]
    fn i8_tests(i: i8) -> bool {
        FromProperty::from_property(ToProperty::to_property(i)) == Ok(i)
    }

    #[quickcheck]
    fn i16_tests(i: i16) -> bool {
        FromProperty::from_property(ToProperty::to_property(i)) == Ok(i)
    }

    #[quickcheck]
    fn i32_tests(i: i32) -> bool {
        FromProperty::from_property(ToProperty::to_property(i)) == Ok(i)
    }

    #[quickcheck]
    fn i64_tests(i: i64) -> bool {
        FromProperty::from_property(ToProperty::to_property(i)) == Ok(i)
    }

    #[quickcheck]
    fn i128_tests(i: i128) -> bool {
        FromProperty::from_property(ToProperty::to_property(i)) == Ok(i)
    }

    #[quickcheck]
    fn f32_tests(i: f32) -> bool {
        !i.is_finite() || FromProperty::from_property(ToProperty::to_property(i)) == Ok(i)
    }

    #[quickcheck]
    fn f64_tests(i: f64) -> bool {
        !i.is_finite() || FromProperty::from_property(ToProperty::to_property(i)) == Ok(i)
    }

    #[quickcheck]
    fn i64_convert_tests(i: i64) -> bool {
        let u8: Result<u8, PropertyError> = FromProperty::from_property(Property::Int(i));
        let u16: Result<u16, PropertyError> = FromProperty::from_property(Property::Int(i));
        let u32: Result<u32, PropertyError> = FromProperty::from_property(Property::Int(i));
        let u64: Result<u64, PropertyError> = FromProperty::from_property(Property::Int(i));
        let u128: Result<u128, PropertyError> = FromProperty::from_property(Property::Int(i));
        let i8: Result<i8, PropertyError> = FromProperty::from_property(Property::Int(i));
        let i16: Result<i16, PropertyError> = FromProperty::from_property(Property::Int(i));
        let i32: Result<i32, PropertyError> = FromProperty::from_property(Property::Int(i));
        let i64: Result<i64, PropertyError> = FromProperty::from_property(Property::Int(i));
        let i128: Result<i128, PropertyError> = FromProperty::from_property(Property::Int(i));
        let f32: Result<f32, PropertyError> = FromProperty::from_property(Property::Int(i));
        let f64: Result<f64, PropertyError> = FromProperty::from_property(Property::Int(i));
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
        let u8: Result<u8, PropertyError> = FromProperty::from_property(Property::Float(i));
        let u16: Result<u16, PropertyError> = FromProperty::from_property(Property::Float(i));
        let u32: Result<u32, PropertyError> = FromProperty::from_property(Property::Float(i));
        let u64: Result<u64, PropertyError> = FromProperty::from_property(Property::Float(i));
        let u128: Result<u128, PropertyError> = FromProperty::from_property(Property::Float(i));
        let i8: Result<i8, PropertyError> = FromProperty::from_property(Property::Float(i));
        let i16: Result<i16, PropertyError> = FromProperty::from_property(Property::Float(i));
        let i32: Result<i32, PropertyError> = FromProperty::from_property(Property::Float(i));
        let i64: Result<i64, PropertyError> = FromProperty::from_property(Property::Float(i));
        let i128: Result<i128, PropertyError> = FromProperty::from_property(Property::Float(i));
        let f32: Result<f32, PropertyError> = FromProperty::from_property(Property::Float(i));
        let f64: Result<f64, PropertyError> = FromProperty::from_property(Property::Float(i));

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
}
