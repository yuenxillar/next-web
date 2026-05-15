use std::sync::Arc;

use crate::core::authentication::Authentication;

#[derive(Clone, Default)]
pub struct SecurityContext {
    authentication: Option<Arc<dyn Authentication>>,
}

impl SecurityContext {
    pub fn new(authentication: Option<Arc<dyn Authentication>>) -> Self {
        Self { authentication }
    }

    pub fn get_authentication(&self) -> Option<Arc<dyn Authentication>> {
        self.authentication.clone()
    }

    pub fn set_authentication(&mut self, authentication: Option<Arc<dyn Authentication>>) {
        self.authentication = authentication;
    }
}
