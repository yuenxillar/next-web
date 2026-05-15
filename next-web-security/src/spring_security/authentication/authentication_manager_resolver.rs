use std::sync::Arc;

use crate::authorization::authentication_manager::AuthenticationManager;

pub trait AuthenticationManagerResolver<C>: Send + Sync {
    fn resolve(&self, context: &C) -> Arc<dyn AuthenticationManager>;
}

#[derive(Clone)]
pub struct FixedAuthenticationManagerResolver {
    authentication_manager: Arc<dyn AuthenticationManager>,
}

impl FixedAuthenticationManagerResolver {
    pub fn new(authentication_manager: Arc<dyn AuthenticationManager>) -> Self {
        Self {
            authentication_manager,
        }
    }
}

impl<C> AuthenticationManagerResolver<C> for FixedAuthenticationManagerResolver {
    fn resolve(&self, _context: &C) -> Arc<dyn AuthenticationManager> {
        self.authentication_manager.clone()
    }
}
