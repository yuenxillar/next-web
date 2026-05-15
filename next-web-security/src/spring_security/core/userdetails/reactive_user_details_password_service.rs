use std::sync::Arc;

use next_web_core::async_trait;

use crate::core::userdetails::user_details::UserDetails;

/// Reactive API for changing a UserDetails password.
#[async_trait]
pub trait ReactiveUserDetailsPasswordService: Send + Sync {
    /// Modify the specified user's password in the persistent repository.
    async fn update_password(
        &self,
        user: Arc<dyn UserDetails>,
        new_password: Option<String>,
    ) -> Arc<dyn UserDetails>;
}

/// No-op implementation that returns the user unchanged.
#[derive(Clone, Default)]
pub struct NoopReactiveUserDetailsPasswordService;

#[async_trait]
impl ReactiveUserDetailsPasswordService for NoopReactiveUserDetailsPasswordService {
    async fn update_password(
        &self,
        user: Arc<dyn UserDetails>,
        _new_password: Option<String>,
    ) -> Arc<dyn UserDetails> {
        user
    }
}
