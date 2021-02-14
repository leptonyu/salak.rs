use crate::*;

/// Convert value from `Property`.
pub trait FromProperty: Sized {
    fn from_property(_: Property) -> Result<Self, PropertyError>;
}

impl FromProperty for Property {
    fn from_property(a: Property) -> Result<Self, PropertyError> {
        Ok(a)
    }
}

impl FromProperty for String {
    fn from_property(p: Property) -> Result<Self, PropertyError> {
        match p {
            Property::Str(str) => Ok(str),
            Property::Int(i64) => Ok(i64.to_string()),
            Property::Float(f64) => Ok(f64.to_string()),
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
                    Property::Int(i64) => Ok(i64 as $x),
                    Property::Float(f64) => Ok(f64 as $x),
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

impl_from_property!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64);

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
            Property::Float(f64) => Ok(f64 != 0.0),
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
}
