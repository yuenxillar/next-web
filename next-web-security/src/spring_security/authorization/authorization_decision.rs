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

impl AuthorizationResult for AuthorizationDecision {
    fn is_granted(&self) -> bool {
        self.granted
    }
}
