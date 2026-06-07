use std::sync::Arc;

use next_web_core::async_trait;

use crate::core::{Authentication, authentication_error::AuthenticationError};

#[async_trait]
pub trait ReactiveAuthenticationManager: Send + Sync {
    async fn authenticate(
        &self,
        authentication: &dyn Authentication,
    ) -> Result<Arc<dyn Authentication>, AuthenticationError>;
}
