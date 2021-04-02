#[allow(unused_imports)]
use crate::*;
use core::num::{ParseFloatError, ParseIntError, TryFromIntError};
use core::str::ParseBoolError;
use std::convert::Infallible;
use std::fmt::{Display, Error, Formatter};

/// Property Error
#[derive(Debug, PartialEq, Eq)]
pub enum PropertyError {
    /// [`Property`] not found
    NotFound(String),
    /// [`Property`] parse failed.
    ParseFail(String),
    /// Recursive parsing same key.
    RecursiveParse(String),
    /// [`PropertySource`] reload failed.
    ReloadFail(String),
    /// Recursive build instance.
    RecursiveBuild(String),
}

impl PropertyError {
    /// Generate parse fail error.
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
            PropertyError::ReloadFail(e) => write!(f, "Reload failed: {}", e),
            PropertyError::RecursiveBuild(e) => write!(f, "Recursive build failed: {}", e),
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

impl From<std::io::Error> for PropertyError {
    fn from(err: std::io::Error) -> Self {
        Self::ParseFail(err.to_string())
    }
}
