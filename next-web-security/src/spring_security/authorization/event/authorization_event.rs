use std::sync::Arc;

use crate::{
    authorization::authorization_result::AuthorizationResult,
    core::authentication::Authentication,
};

/// Base event for authorization results.
#[derive(Clone)]
pub struct AuthorizationEvent {
    authentication: Arc<dyn Authentication>,
    secured_object_description: String,
    result: Arc<dyn AuthorizationResult>,
}

impl AuthorizationEvent {
    pub fn new(
        authentication: Arc<dyn Authentication>,
        secured_object_description: impl Into<String>,
        result: Arc<dyn AuthorizationResult>,
    ) -> Self {
        Self {
            authentication,
            secured_object_description: secured_object_description.into(),
            result,
        }
    }

    pub fn authentication(&self) -> Arc<dyn Authentication> {
        self.authentication.clone()
    }

    pub fn secured_object_description(&self) -> &str {
        &self.secured_object_description
    }

    pub fn authorization_result(&self) -> Arc<dyn AuthorizationResult> {
        self.result.clone()
    }
}
