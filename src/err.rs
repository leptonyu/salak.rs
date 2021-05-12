use std::error::Error;

#[allow(unused_imports)]
use crate::*;

/// Property Error
#[derive(Debug)]
pub enum PropertyError {
    /// [`Property`] parse failed.
    ParseFail(Option<Box<dyn Error>>),
    /// Resolve fail.
    ResolveFail,
    /// [`Property`] not found when resolve.
    ResolveNotFound(String),
    /// Recursive parsing same key.
    RecursiveFail(String),
    /// [`Property`] not found
    NotFound(String),
}

impl<E: Error + 'static> From<E> for PropertyError {
    fn from(err: E) -> Self {
        PropertyError::ParseFail(Some(Box::new(err)))
    }
}
