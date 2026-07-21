use std::sync::Arc;

use next_web_core::BoxAny;

use crate::{authorization::authorization_result::AuthorizationResult, core::Authentication};

pub trait AuthorizationEventPublisher
where
    Self: Send + Sync,
{
    fn publish_authorization_event(
        &self,
        authentication: Arc<dyn Authentication>,
        value: BoxAny,
        result: Option<Box<dyn AuthorizationResult>>,
    );
}
