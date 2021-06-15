use std::net::Shutdown;

#[cfg(feature = "log")]
use log::*;

use crate::*;

/// Any enum implements this trait is automatically implementing [`IsProperty`].
pub trait EnumProperty: Sized {
    /// Convert str to enum.
    fn str_to_enum(val: &str) -> Res<Self>;
}

impl<T: EnumProperty> IsProperty for T {
    #[inline]
    fn from_property(p: Property<'_>) -> Res<Self> {
        match p {
            Property::S(v) => T::str_to_enum(v),
            Property::O(v) => T::str_to_enum(&v),
            _ => Err(PropertyError::parse_fail("only string can convert to enum")),
        }
    }
}

/// Implement enum as [`EnumProperty`]
#[macro_export]
macro_rules! impl_enum_property {
    ($x:path {$($k:literal => $v:expr)+ }) => {
        impl $crate::EnumProperty for $x {
            fn str_to_enum(val: &str) -> Result<$x, $crate::PropertyError> {
                match &val.to_lowercase()[..] {
                    $($k => Ok($v),)+
                    _ => Err($crate::PropertyError::parse_fail("invalid enum value")),
                }
            }
        }
    }
}

#[cfg(feature = "log")]
#[cfg_attr(docsrs, doc(cfg(feature = "log")))]
impl_enum_property!(LevelFilter {
  "off"   => LevelFilter::Off
  "error" => LevelFilter::Error
  "warn"  => LevelFilter::Warn
  "info"  => LevelFilter::Info
  "debug" => LevelFilter::Debug
  "trace" => LevelFilter::Trace
});

#[cfg(feature = "log")]
#[cfg_attr(docsrs, doc(cfg(feature = "log")))]
impl_enum_property!(Level {
  "error" => Level::Error
  "warn"  => Level::Warn
  "info"  => Level::Info
  "debug" => Level::Debug
  "trace" => Level::Trace
});

impl_enum_property!(Shutdown{
    "read" => Shutdown::Read
    "write" => Shutdown::Write
    "both" => Shutdown::Both
});
