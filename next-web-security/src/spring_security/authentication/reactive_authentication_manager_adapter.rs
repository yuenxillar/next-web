use std::sync::Arc;

use next_web_core::async_trait;

use crate::{
    authentication::reactive_authentication_manager::ReactiveAuthenticationManager,
    authorization::AuthenticationManager,
    core::{Authentication, authentication_error::AuthenticationError},
};

pub struct ReactiveAuthenticationManagerAdapter {
    authentication_manager: Arc<dyn AuthenticationManager>,
}

impl ReactiveAuthenticationManagerAdapter {
    pub fn new(authentication_manager: Arc<dyn AuthenticationManager>) -> Self {
        Self {
            authentication_manager,
        }
    }
}

#[async_trait]
impl ReactiveAuthenticationManager for ReactiveAuthenticationManagerAdapter {
    async fn authenticate(
        &self,
        authentication: &dyn Authentication,
    ) -> Result<Arc<dyn Authentication>, AuthenticationError> {
        self.authentication_manager.authenticate(authentication)
    }
}
