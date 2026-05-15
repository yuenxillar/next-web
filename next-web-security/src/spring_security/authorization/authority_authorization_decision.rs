#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthorityAuthorizationDecision {
    granted: bool,
    authorities: Vec<String>,
}

impl AuthorityAuthorizationDecision {
    pub fn new(granted: bool, authorities: Vec<String>) -> Self {
        Self {
            granted,
            authorities,
        }
    }

    pub fn is_granted(&self) -> bool {
        self.granted
    }

    pub fn authorities(&self) -> &[String] {
        &self.authorities
    }
}
