

pub struct AuthorizationDecision {
    granted: bool
}


impl AuthorizationDecision {
    
    pub fn new(granted: bool) -> Self {
        Self { granted }
    }
    
    pub fn is_granted(&self) -> bool {
        true
    }
}