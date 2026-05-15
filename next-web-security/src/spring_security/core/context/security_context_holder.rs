use std::sync::{Arc, OnceLock, RwLock};

use crate::core::context::{
    security_context::SecurityContext,
    security_context_holder_strategy::SecurityContextHolderStrategy,
    thread_local_security_context_holder_strategy::ThreadLocalSecurityContextHolderStrategy,
};

fn strategy_cell() -> &'static RwLock<Arc<dyn SecurityContextHolderStrategy>> {
    static STRATEGY: OnceLock<RwLock<Arc<dyn SecurityContextHolderStrategy>>> = OnceLock::new();
    STRATEGY.get_or_init(|| {
        RwLock::new(Arc::new(ThreadLocalSecurityContextHolderStrategy) as Arc<_>)
    })
}

pub struct SecurityContextHolder;

impl SecurityContextHolder {
    pub fn clear_context() {
        Self::get_context_holder_strategy().clear_context();
    }

    pub fn get_context() -> SecurityContext {
        Self::get_context_holder_strategy().get_context()
    }

    pub fn set_context(context: SecurityContext) {
        Self::get_context_holder_strategy().set_context(context);
    }

    pub fn create_empty_context() -> SecurityContext {
        Self::get_context_holder_strategy().create_empty_context()
    }

    pub fn get_context_holder_strategy() -> Arc<dyn SecurityContextHolderStrategy> {
        strategy_cell()
            .read()
            .expect("security context holder strategy lock poisoned")
            .clone()
    }

    pub fn set_context_holder_strategy(strategy: Arc<dyn SecurityContextHolderStrategy>) {
        *strategy_cell()
            .write()
            .expect("security context holder strategy lock poisoned") = strategy;
    }

    pub fn reset_to_thread_local_strategy() {
        Self::set_context_holder_strategy(Arc::new(ThreadLocalSecurityContextHolderStrategy));
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::core::{
        authentication::Authentication,
        context::{security_context::SecurityContext, security_context_holder::SecurityContextHolder},
        username_password_authentication_token::UsernamePasswordAuthenticationToken,
    };

    #[test]
    fn holder_stores_and_clears_context() {
        SecurityContextHolder::reset_to_thread_local_strategy();
        SecurityContextHolder::clear_context();

        let token = UsernamePasswordAuthenticationToken::unauthenticated(
            Some(String::from("alice")),
            Some(String::from("secret")),
        );
        let context = SecurityContext::new(Some(Arc::new(token) as Arc<dyn Authentication>));

        SecurityContextHolder::set_context(context);
        let current = SecurityContextHolder::get_context();
        assert_eq!(
            current
                .get_authentication()
                .as_ref()
                .and_then(|authentication| authentication.get_principal()),
            Some(String::from("alice"))
        );

        SecurityContextHolder::clear_context();
        assert!(SecurityContextHolder::get_context()
            .get_authentication()
            .is_none());
    }
}
