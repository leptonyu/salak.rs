use std::ops::DerefMut;

#[cfg(feature = "derive")]
use crate::SalakDescContext;
use crate::{FromEnvironment, Property, PropertyError, SalakContext};

/// A wrapper of [`Vec<T>`], but require having at least one value when parsing configuration.
#[derive(Debug)]
pub struct NonEmptyVec<T>(pub Vec<T>);

impl<T> std::ops::Deref for NonEmptyVec<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for NonEmptyVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: FromEnvironment> FromEnvironment for NonEmptyVec<T> {
    fn from_env(
        val: Option<Property<'_>>,
        env: &mut SalakContext<'_>,
    ) -> Result<Self, PropertyError> {
        let v = <Vec<T>>::from_env(val, env)?;
        if v.is_empty() {
            return Err(PropertyError::NotFound(env.current_key().to_string()));
        }
        Ok(NonEmptyVec(v))
    }

    #[cfg(feature = "derive")]
    #[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
    fn key_desc(env: &mut SalakDescContext<'_>) {
        env.current.set_required(true);
        <Vec<T>>::key_desc(env);
    }
}
