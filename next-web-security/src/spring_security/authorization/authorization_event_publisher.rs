use std::sync::Arc;

use crate::{authorization::authorization_result::AuthorizationResult, core::Authentication};

pub trait AuthorizationEventPublisher<T = ()>
where
    Self: Send + Sync,
{
    fn publish_authorization_event(
        &self,
        authentication: Arc<dyn Authentication>,
        var: T,
        result: Option<Box<dyn AuthorizationResult>>,
    );
}
