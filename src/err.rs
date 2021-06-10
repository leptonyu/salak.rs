use std::{error::Error, fmt::Display};

#[allow(unused_imports)]
use crate::*;

/// Property error for the whole crate.
#[derive(Debug)]
pub enum PropertyError {
    /// [`Property`] parse failed.
    ParseFail(Option<String>, Box<dyn Error>),
    /// Resolve fail.
    ResolveFail(String),
    /// [`Property`] not found when resolve.
    ResolveNotFound(String),
    /// Recursive parsing same key.
    RecursiveFail(String),
    /// [`Property`] not found
    NotFound(String),
    /// Resource not found
    ResourceNotFound(&'static str, &'static str),
    /// Resource already registered.
    ResourceRegistered(&'static str, &'static str),
    /// Resource recursive dependent.
    ResourceRecursive(&'static str, &'static str),
}

#[derive(Debug)]
/// Salak parse error.
pub(crate) struct SalakParseError(String);

impl Display for SalakParseError {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl Error for SalakParseError {}

impl PropertyError {
    /// Create parse fail error.
    #[inline]
    pub fn parse_fail(msg: &str) -> Self {
        PropertyError::ParseFail(None, Box::new(SalakParseError(msg.to_string())))
    }
}

impl<E: Error + 'static> From<E> for PropertyError {
    #[inline]
    fn from(err: E) -> Self {
        PropertyError::ParseFail(None, Box::new(err))
    }
}
