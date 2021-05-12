use crate::{Property, PropertyError};

pub trait IsProperty: Sized {
    fn is_empty(p: &Property<'_>) -> bool {
        match p {
            Property::S(s) => s.is_empty(),
            Property::O(s) => s.is_empty(),
            _ => false,
        }
    }
    fn from_property(_: Property<'_>) -> Result<Self, PropertyError>;
}

impl IsProperty for String {
    fn is_empty(p: &Property<'_>) -> bool {
        false
    }
    fn from_property(p: Property<'_>) -> Result<Self, PropertyError> {
        Ok(match p {
            Property::S(v) => v.to_string(),
            Property::O(v) => v,
            Property::I(v) => v.to_string(),
            Property::F(v) => v.to_string(),
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
                _ => Err(PropertyError::ParseFail(None)),
            }
        }
        match p {
            Property::B(v) => Ok(v),
            Property::S(v) => str_to_bool(v),
            Property::O(v) => str_to_bool(&v),
            _ => Err(PropertyError::ParseFail(None)),
        }
    }
}

macro_rules! impl_property_num {
    ($($x:ident),+) => {$(
            impl IsProperty for $x {
                fn from_property(p: Property<'_>) -> Result<Self, PropertyError> {
                    Ok(match p {
                    Property::S(s) => s.parse::<$x>()?,
                    Property::O(s) => s.parse::<$x>()?,
                    Property::I(s) => s as $x,
                    Property::F(s) => s as $x,
                    _ => Err(PropertyError::ParseFail(None))?,
                    })
                }

            }

            )+}
}

impl_property_num!(i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, isize, usize);
