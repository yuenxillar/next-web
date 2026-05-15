use std::sync::Arc;

use crate::{
    authorization::authorization_result::AuthorizationResult,
    core::authentication::Authentication,
};

pub trait AuthorizationEventPublisher
where
    Self: Send + Sync,
{
    fn publish_authorization_event(
        &self,
        authentication: Arc<dyn Authentication>,
        object_description: &str,
        result: Option<Arc<dyn AuthorizationResult>>,
    );
}
