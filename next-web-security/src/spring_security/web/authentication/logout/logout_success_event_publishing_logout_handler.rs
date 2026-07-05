use std::sync::Arc;

use next_web_context::ApplicationEventPublisher;
use next_web_core::{
    async_trait,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::{
    authentication::event::LogoutSuccessEvent, core::Authentication,
    web::authentication::logout::LogoutHandler,
};

/// A logout handler which publishes LogoutSuccessEvent
#[derive(Default, Clone)]
pub struct LogoutSuccessEventPublishingLogoutHandler {
    event_publisher: Option<Arc<dyn ApplicationEventPublisher>>,
}

#[async_trait]
impl LogoutHandler for LogoutSuccessEventPublishingLogoutHandler {
    async fn logout(
        &self,
        _request: &mut dyn HttpRequest,
        _response: &mut dyn HttpResponse,
        authentication: Option<&Arc<dyn Authentication>>,
    ) {
        match self.event_publisher.as_ref() {
            Some(event_publisher) => {
                if let Some(auth) = authentication {
                    event_publisher
                        .publish_event(Box::new(LogoutSuccessEvent::new(auth.to_owned())))
                        .await
                        .ok();
                }
            }
            None => return,
        }
    }
}
