#[cfg(feature = "log")]
use log::*;

use crate::*;

/// Implement enum as [`IsProperty`]
#[macro_export]
macro_rules! impl_enum_property {
    ($x:ident {$($k:literal => $v:path)+ }) => {
        impl IsProperty for $x {
            fn from_property(p: Property<'_>) -> Result<Self, PropertyError> {
                #[inline]
                fn str_to_enum(val: &str) -> Result<$x, PropertyError> {
                    match &val.to_lowercase()[..] {
                        $($k => Ok($v),)+
                        _ => Err(PropertyError::parse_fail("invalid enum value")),
                    }
                }
                match p {
                    Property::S(v) => str_to_enum(v),
                    Property::O(v) => str_to_enum(&v),
                    _ => Err(PropertyError::parse_fail("only string can convert to enum")),
                }
            }
        }
    }
}

#[cfg(feature = "log")]
impl_enum_property!(LevelFilter {
  "off"   => LevelFilter::Off
  "error" => LevelFilter::Error
  "warn"  => LevelFilter::Warn
  "info"  => LevelFilter::Info
  "debug" => LevelFilter::Debug
  "trace" => LevelFilter::Trace
});

#[cfg(feature = "log")]
impl_enum_property!(Level {
  "error" => Level::Error
  "warn"  => Level::Warn
  "info"  => Level::Info
  "debug" => Level::Debug
  "trace" => Level::Trace
});
