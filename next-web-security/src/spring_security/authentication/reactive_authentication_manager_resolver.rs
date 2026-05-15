use std::sync::Arc;

use crate::authentication::reactive_authentication_manager::ReactiveAuthenticationManager;

pub trait ReactiveAuthenticationManagerResolver<C>: Send + Sync {
    fn resolve(&self, context: &C) -> Arc<dyn ReactiveAuthenticationManager>;
}

#[derive(Clone)]
pub struct FixedReactiveAuthenticationManagerResolver {
    authentication_manager: Arc<dyn ReactiveAuthenticationManager>,
}

impl FixedReactiveAuthenticationManagerResolver {
    pub fn new(authentication_manager: Arc<dyn ReactiveAuthenticationManager>) -> Self {
        Self {
            authentication_manager,
        }
    }
}

impl<C> ReactiveAuthenticationManagerResolver<C> for FixedReactiveAuthenticationManagerResolver {
    fn resolve(&self, _context: &C) -> Arc<dyn ReactiveAuthenticationManager> {
        self.authentication_manager.clone()
    }
}
