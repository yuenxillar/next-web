use std::sync::Arc;

use crate::authorization::AuthenticationManager;

pub struct AuthenticationConfiguration {}

impl AuthenticationConfiguration {
    pub fn get_authentication_manager(&self) -> Arc<dyn AuthenticationManager> {
        todo!()
    }
}
