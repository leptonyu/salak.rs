use crate::*;

#[doc(hidden)]
pub trait AutoDeriveFromEnvironment: FromEnvironment {}

impl<P: AutoDeriveFromEnvironment> AutoDeriveFromEnvironment for Option<P> {}

#[doc(hidden)]
pub trait DefaultSourceFromEnvironment: AutoDeriveFromEnvironment {
    fn prefix() -> &'static str;
}

impl<P: DefaultSourceFromEnvironment> DefaultSourceFromEnvironment for Option<P> {
    fn prefix() -> &'static str {
        P::prefix()
    }
}
