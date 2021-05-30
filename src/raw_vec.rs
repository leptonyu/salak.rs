use std::ops::DerefMut;

#[cfg(feature = "derive")]
use crate::{DescFromEnvironment, SalakDescContext};
use crate::{FromEnvironment, Property, PropertyError, SalakContext};

/// A wrapper of [`Vec<T>`], but require having at least one value when parsing configuration.
#[derive(Debug)]
pub struct NonEmptyVec<T>(Vec<T>);

impl<T> NonEmptyVec<T> {
    /// Get [`Vec<T>`].
    #[inline]
    pub fn into_vec(self) -> Vec<T> {
        self.0
    }
}

impl<T> IntoIterator for NonEmptyVec<T> {
    type Item = T;

    type IntoIter = std::vec::IntoIter<T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<T> std::ops::Deref for NonEmptyVec<T> {
    type Target = Vec<T>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> Into<Vec<T>> for NonEmptyVec<T> {
    #[inline]
    fn into(self) -> Vec<T> {
        self.into_vec()
    }
}

impl<T> DerefMut for NonEmptyVec<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: FromEnvironment> FromEnvironment for NonEmptyVec<T> {
    #[inline]
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
}

#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
impl<T: DescFromEnvironment> DescFromEnvironment for NonEmptyVec<T> {
    fn key_desc(env: &mut SalakDescContext<'_>) {
        env.current.set_required(true);
        <Vec<T>>::key_desc(env);
    }
}
