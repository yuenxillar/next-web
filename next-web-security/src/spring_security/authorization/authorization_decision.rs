use std::ops::Deref;

use crate::authorization::AuthorizationResult;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthorizationDecision {
    granted: bool,
}

impl AuthorizationDecision {
    pub fn new(granted: bool) -> Self {
        Self { granted }
    }
}

impl<T> AuthorizationResult for T
where
    T: Deref<Target = AuthorizationDecision>,
    T: Send + Sync,
    T: Clone,
    T: 'static,
{
    fn is_granted(&self) -> bool {
        self.granted
    }
}
