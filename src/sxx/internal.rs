//! [`Property`] converter.
use crate::*;
use core::convert::TryFrom;
use regex::*;
use std::{collections::HashMap, marker::PhantomData};
use std::{convert::TryInto, time::Duration};

impl Into<Property> for String {
    fn into(self) -> Property {
        Property::Str(self)
    }
}

impl Into<Property> for &str {
    fn into(self) -> Property {
        Property::Str(self.to_owned())
    }
}

macro_rules! impl_into_property {
    ($($x:ident),+) => {$(
        impl Into<Property> for $x {
            #[allow(trivial_numeric_casts)]
            fn into(self) -> Property {
                Property::Int(self as i64)
            }
        }
    )+};
}

impl_into_property!(u8, u16, u32, i8, i16, i32, i64);

macro_rules! impl_into_property_str {
    ($($x:ident),+) => {$(
        impl Into<Property> for $x {
            fn into(self) -> Property {
                Property::Str(self.to_string())
            }
        }
    )+};
}

impl_into_property_str!(u64, u128, i128, isize, usize);

macro_rules! impl_float_into_property {
    ($($x:ident),+) => {$(
        impl Into<Property> for $x {
            #[allow(trivial_numeric_casts)]
            fn into(self) -> Property {
                Property::Float(self as f64)
            }
        }
    )+};
}

impl_float_into_property!(f32, f64);

fn check_f64(f: f64) -> Result<f64, PropertyError> {
    if f.is_finite() {
        Ok(f)
    } else {
        Err(PropertyError::parse_failed("f64 value is infinite"))
    }
}

impl FromEnvironment for Property {
    fn from_env(
        n: &str,
        property: Option<Property>,
        _: &impl Environment,
    ) -> Result<Self, PropertyError> {
        if let Some(p) = property {
            return Ok(p);
        }
        Self::from_err(PropertyError::NotFound(n.to_owned()))
    }
}

macro_rules! impl_from_environment {
    ($($x:ident),+) => {$(
        impl FromEnvironment for $x {
            fn from_env(
                n: &str,
                property: Option<Property>,
                _: &impl Environment,
            ) -> Result<Self, PropertyError> {
                if let Some(p) = property {
                    return p.try_into();
                }
                Self::from_err(PropertyError::NotFound(n.to_owned()))
            }
        }
    )+}
}

impl TryInto<String> for Property {
    type Error = PropertyError;
    fn try_into(self) -> Result<String, PropertyError> {
        match self {
            Property::Str(str) => {
                if str.is_empty() {
                    Err(PropertyError::NotFound("".to_string()))
                } else {
                    Ok(str)
                }
            }
            Property::Int(i64) => Ok(i64.to_string()),
            Property::Float(f64) => Ok(check_f64(f64)?.to_string()),
            Property::Bool(bool) => Ok(bool.to_string()),
        }
    }
}

impl_from_environment!(char, String);

impl TryInto<char> for Property {
    type Error = PropertyError;
    fn try_into(self) -> Result<char, PropertyError> {
        match self {
            Property::Str(str) => {
                let mut chars = str.chars();
                if let Some(c) = chars.next() {
                    if chars.next().is_none() {
                        return Ok(c);
                    }
                }
                Err(PropertyError::parse_failed("Invalid char value"))
            }
            Property::Int(_) => Err(PropertyError::parse_failed(
                "Integer value cannot convert to char",
            )),
            Property::Float(_) => Err(PropertyError::parse_failed(
                "Float value cannot convert to char",
            )),
            Property::Bool(_) => Err(PropertyError::parse_failed(
                "Bool value cannot convert to char",
            )),
        }
    }
}

macro_rules! impl_from_property {
    ($($x:ident),+) => {$(
        impl TryInto<$x> for Property {
            type Error = PropertyError;
            fn try_into(self) -> Result<$x, PropertyError> {
                match self {
                    Property::Str(str) => if str.is_empty() {
                        Err(PropertyError::NotFound("".to_string()))
                    } else {
                        Ok(str.parse::<$x>()?)
                    },
                    Property::Int(i64) => Ok(<$x>::try_from(i64)?),
                    Property::Float(f64) => Ok(check_f64(f64)? as $x),
                    Property::Bool(_) => Err(PropertyError::ParseFail(
                        format!("Bool value cannot convert to {}",stringify!($x)),
                    )),
                }
            }
        }
        impl_from_environment!($x);
    )+}
}

impl_from_property!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);

macro_rules! impl_float_from_property {
    ($($x:ident),+) => {$(
        impl TryInto<$x> for Property {
            type Error = PropertyError;
            #[allow(trivial_numeric_casts)]
            fn try_into(self) -> Result<$x, PropertyError> {
                match self {
                    Property::Str(str) => if str.is_empty() {
                        Err(PropertyError::NotFound("".to_string()))
                    } else {
                        Ok(str.parse::<$x>()?)
                    },
                    Property::Int(i64) => Ok(i64 as $x),
                    Property::Float(f64) => Ok(check_f64(f64)? as $x),
                    Property::Bool(_) => Err(PropertyError::ParseFail(
                        format!("Bool value cannot convert to {}",stringify!($x)),
                    )),
                }
            }
        }
        impl_from_environment!($x);
    )+}
}

impl_float_from_property!(f64, f32);

impl TryInto<bool> for Property {
    type Error = PropertyError;
    fn try_into(self) -> Result<bool, PropertyError> {
        lazy_static::lazy_static! {
        static ref STR_YES: HashSet<String> = vec!["yes","y","1","true","t"].into_iter().map(|a|a.to_string()).collect();
        static ref STR_NO: HashSet<String> = vec!["no","n","0","false","f"].into_iter().map(|a|a.to_string()).collect();
        }
        match self {
            Property::Str(str) => {
                let str = str.to_lowercase();
                if STR_YES.contains(&str) {
                    Ok(true)
                } else if STR_NO.contains(&str) {
                    Ok(false)
                } else if str.is_empty() {
                    Err(PropertyError::NotFound("".to_string()))
                } else {
                    Err(PropertyError::parse_failed("Str cannot convert to bool"))
                }
            }
            Property::Int(i64) => Ok(i64 != 0),
            Property::Float(f64) => Ok(check_f64(f64)? != 0.0),
            Property::Bool(bool) => Ok(bool),
        }
    }
}
impl_from_environment!(bool);

impl TryInto<Duration> for Property {
    type Error = PropertyError;
    fn try_into(self) -> Result<Duration, PropertyError> {
        match self {
            Property::Str(du) => parse_duration_from_str(&du),
            Property::Int(seconds) => Ok(Duration::new(seconds as u64, 0)),
            Property::Float(sec) => Ok(Duration::new(0, 0).mul_f64(sec)),
            Property::Bool(_) => Err(PropertyError::parse_failed(
                "Datetime only support string value parse.",
            )),
        }
    }
}
impl_from_environment!(Duration);

const NS: u32 = 1_000_000_000;

fn parse_duration_from_str(du: &str) -> Result<Duration, PropertyError> {
    lazy_static::lazy_static! {
        static ref RE: Regex = Regex::new(
            r"^([0-9]+)(h|m|s|ms|us|ns)?$"
        )
        .expect(NOT_POSSIBLE);
        static ref ML: HashMap<String, (u64,u32)> = vec![("h",(3600,0))
        ,("m",(60,0))
        ,("s",(1,0))
        ,("ms",(0, 1_000_000))
        ,("us",(0, 1000))
        ,("ns",(0, 1))]
            .into_iter()
            .map(|(k,v)|(k.to_owned(),v))
            .collect();
    }
    if du.is_empty() {
        return Err(PropertyError::NotFound("".to_string()));
    }
    match RE.captures(du) {
        Some(ref cap) => {
            let unit = cap.get(2).map(|r| r.as_str()).unwrap_or("s");
            let (a, b) = ML.get(unit).unwrap_or(&(1, 0));
            let i: u64 = cap.get(1).expect(NOT_POSSIBLE).as_str().parse()?;
            let mut a = a * i;
            let mut b = b * (i as u32);
            if b > NS {
                a += (b / NS) as u64;
                b %= NS;
            }
            Ok(Duration::new(a, b))
        }
        _ => Err(PropertyError::parse_failed("Invalid duration format")),
    }
}

impl<T> TryInto<PhantomData<T>> for Property {
    type Error = PropertyError;
    fn try_into(self) -> Result<PhantomData<T>, PropertyError> {
        Ok(PhantomData)
    }
}

#[cfg(test)]
mod tests {
    use std::convert::TryInto;

    use source::internal::parse_duration_from_str;

    use crate::*;
    #[test]
    fn bool_tests() {
        assert_eq!(Ok(true), Property::Str("yes".to_owned()).try_into());
        assert_eq!(Ok(true), Property::Str("y".to_owned()).try_into());
        assert_eq!(Ok(true), Property::Str("1".to_owned()).try_into());
        assert_eq!(Ok(true), Property::Str("true".to_owned()).try_into());
        assert_eq!(Ok(true), Property::Str("t".to_owned()).try_into());
        assert_eq!(Ok(true), Property::Int(1).try_into());
        assert_eq!(Ok(true), Property::Float(1.0).try_into());
        assert_eq!(Ok(true), Property::Bool(true).try_into());

        assert_eq!(Ok(false), Property::Str("no".to_owned()).try_into());
        assert_eq!(Ok(false), Property::Str("n".to_owned()).try_into());
        assert_eq!(Ok(false), Property::Str("0".to_owned()).try_into());
        assert_eq!(Ok(false), Property::Str("false".to_owned()).try_into());
        assert_eq!(Ok(false), Property::Str("f".to_owned()).try_into());
        assert_eq!(Ok(false), Property::Int(0).try_into());
        assert_eq!(Ok(false), Property::Float(0.0).try_into());
        assert_eq!(Ok(false), Property::Bool(false).try_into());

        let x: Result<bool, PropertyError> = Property::Str("x".to_owned()).try_into();
        assert_eq!(true, x.is_err());
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
        assert_eq!(Ok(i), Property::Str(format!("{}", i)).try_into());
        assert_eq!(Ok(i), Property::Int(i).try_into());
        let v: Result<i64, PropertyError> = Property::Bool(true).try_into();
        assert_eq!(
            Err(PropertyError::parse_failed(
                "Bool value cannot convert to i64"
            )),
            v
        );
    }

    #[quickcheck]
    fn u8_tests(i: u8) -> bool {
        Property::try_into(i.into()) == Ok(i)
    }

    #[quickcheck]
    fn u16_tests(i: u16) -> bool {
        Property::try_into(i.into()) == Ok(i)
    }

    #[quickcheck]
    fn u32_tests(i: u32) -> bool {
        Property::try_into(i.into()) == Ok(i)
    }

    #[quickcheck]
    fn u64_tests(i: u64) -> bool {
        Property::try_into(i.into()) == Ok(i)
    }

    #[quickcheck]
    fn u128_tests(i: u128) -> bool {
        Property::try_into(i.into()) == Ok(i)
    }

    #[quickcheck]
    fn i8_tests(i: i8) -> bool {
        Property::try_into(i.into()) == Ok(i)
    }

    #[quickcheck]
    fn i16_tests(i: i16) -> bool {
        Property::try_into(i.into()) == Ok(i)
    }

    #[quickcheck]
    fn i32_tests(i: i32) -> bool {
        Property::try_into(i.into()) == Ok(i)
    }

    #[quickcheck]
    fn i64_tests(i: i64) -> bool {
        Property::try_into(i.into()) == Ok(i)
    }

    #[quickcheck]
    fn i128_tests(i: i128) -> bool {
        Property::try_into(i.into()) == Ok(i)
    }

    #[quickcheck]
    fn f32_tests(i: f32) -> bool {
        !i.is_finite() || Property::try_into(i.into()) == Ok(i)
    }

    #[quickcheck]
    fn f64_tests(i: f64) -> bool {
        !i.is_finite() || Property::try_into(i.into()) == Ok(i)
    }

    #[quickcheck]
    fn i64_convert_tests(i: i64) -> bool {
        let u8: Result<u8, PropertyError> = Property::Int(i).try_into();
        let u16: Result<u16, PropertyError> = Property::Int(i).try_into();
        let u32: Result<u32, PropertyError> = Property::Int(i).try_into();
        let u64: Result<u64, PropertyError> = Property::Int(i).try_into();
        let u128: Result<u128, PropertyError> = Property::Int(i).try_into();
        let i8: Result<i8, PropertyError> = Property::Int(i).try_into();
        let i16: Result<i16, PropertyError> = Property::Int(i).try_into();
        let i32: Result<i32, PropertyError> = Property::Int(i).try_into();
        let i64: Result<i64, PropertyError> = Property::Int(i).try_into();
        let i128: Result<i128, PropertyError> = Property::Int(i).try_into();
        let f32: Result<f32, PropertyError> = Property::Int(i).try_into();
        let f64: Result<f64, PropertyError> = Property::Int(i).try_into();
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
        let u8: Result<u8, PropertyError> = Property::Float(i).try_into();
        let u16: Result<u16, PropertyError> = Property::Float(i).try_into();
        let u32: Result<u32, PropertyError> = Property::Float(i).try_into();
        let u64: Result<u64, PropertyError> = Property::Float(i).try_into();
        let u128: Result<u128, PropertyError> = Property::Float(i).try_into();
        let i8: Result<i8, PropertyError> = Property::Float(i).try_into();
        let i16: Result<i16, PropertyError> = Property::Float(i).try_into();
        let i32: Result<i32, PropertyError> = Property::Float(i).try_into();
        let i64: Result<i64, PropertyError> = Property::Float(i).try_into();
        let i128: Result<i128, PropertyError> = Property::Float(i).try_into();
        let f32: Result<f32, PropertyError> = Property::Float(i).try_into();
        let f64: Result<f64, PropertyError> = Property::Float(i).try_into();

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
    fn duration_tests() {
        use std::time::Duration;
        assert_eq!(Ok(Duration::new(123, 0)), parse_duration_from_str("123"));
        assert_eq!(Ok(Duration::new(123, 0)), parse_duration_from_str("123s"));
        assert_eq!(
            Ok(Duration::new(10 * 60, 0)),
            parse_duration_from_str("10m")
        );
        assert_eq!(
            Ok(Duration::new(123 * 3600, 0)),
            parse_duration_from_str("123h")
        );
        assert_eq!(
            Ok(Duration::new(0, 123 * 1000_000)),
            parse_duration_from_str("123ms")
        );
        assert_eq!(
            Ok(Duration::new(0, 123 * 1000)),
            parse_duration_from_str("123us")
        );
        assert_eq!(Ok(Duration::new(0, 123)), parse_duration_from_str("123ns"));
        assert_eq!(Ok(Duration::new(1, 0)), parse_duration_from_str("1000ms"));
    }
}
