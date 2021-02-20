use core::num::*;
use core::str::ParseBoolError;
use std::convert::Infallible;
use std::fmt::{Display, Error, Formatter};

/// Property Error
#[derive(Debug, PartialEq, Eq)]
pub enum PropertyError {
    /// Property not found
    NotFound(String),
    /// Property parse failed.
    ParseFail(String),
    /// Resursive parsing same key.
    RecursiveParse(String),
}

impl PropertyError {
    pub fn parse_failed(msg: &str) -> Self {
        Self::ParseFail(msg.to_owned())
    }
}

impl Display for PropertyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match self {
            PropertyError::NotFound(n) => write!(f, "Property {} not found.", n),
            PropertyError::ParseFail(e) => write!(f, "Parse failed: {}", e),
            PropertyError::RecursiveParse(n) => write!(f, "Property {} recursive.", &n),
        }
    }
}

impl std::error::Error for PropertyError {}

macro_rules! impl_parse_failed {
    ($x:ident) => {
        impl From<$x> for PropertyError {
            fn from(e: $x) -> Self {
                Self::ParseFail(e.to_string())
            }
        }
    };
    ($x:ident, $($y:ident),+) => {
        impl_parse_failed!($x);
        impl_parse_failed!($($y),+);
    };
}

impl_parse_failed!(
    ParseBoolError,
    ParseIntError,
    ParseFloatError,
    TryFromIntError,
    Infallible
);

#[cfg(feature = "enable_toml")]
#[cfg_attr(docsrs, doc(cfg(feature = "enable_toml")))]
impl From<::toml::value::DatetimeParseError> for PropertyError {
    fn from(err: ::toml::value::DatetimeParseError) -> Self {
        Self::ParseFail(err.to_string())
    }
}
