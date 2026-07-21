use std::sync::Arc;

use next_web_core::async_trait;

use crate::core::userdetails::UserDetails;

#[async_trait]
pub trait UserDetailsPasswordService: Send + Sync {
    async fn update_password(
        &self,
        user: Arc<dyn UserDetails>,
        new_password: Option<String>,
    ) -> Arc<dyn UserDetails>;
}

#[derive(Clone, Default)]
pub struct NoopUserDetailsPasswordService;

#[async_trait]
impl UserDetailsPasswordService for NoopUserDetailsPasswordService {
    async fn update_password(
        &self,
        user: Arc<dyn UserDetails>,
        _new_password: Option<String>,
    ) -> Arc<dyn UserDetails> {
        user
    }
}
