use std::{ops::Deref, sync::Arc};

use next_web_context::ApplicationEventPublisher;
use next_web_core::{
    async_trait,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse, HttpSession},
};
use tracing::{debug, enabled, warn, Level};

use crate::{
    core::{Authentication, AuthenticationError},
    web::authentication::session::{SessionAuthenticationStrategy, SessionFixationProtectionEvent},
};

// A base class for performing session fixation protection.
#[derive(Clone)]
pub struct BaseSessionFixationProtectionStrategy {
    application_event_publisher: Option<Arc<dyn ApplicationEventPublisher>>,
    always_create_session: bool,
}

impl BaseSessionFixationProtectionStrategy {
    fn get_application_event_publisher(&self) -> Option<&Arc<dyn ApplicationEventPublisher>> {
        self.application_event_publisher.as_ref()
    }

    /// Called when the session has been changed and the old attributes have been migrated
    /// to the new session. Only called if a session existed to start with. Allows
    /// subclasses to plug in additional behaviour.
    ///
    /// The default implementation of this method publishes a
    /// `SessionFixationProtectionEvent` to notify the application that the session ID has
    /// changed. If you override this method and still wish these events to be published,
    /// you should call `super.on_session_change()` within your overriding method.
    ///
    /// # Arguments
    ///
    /// * `original_session_id` - The original session identifier.
    /// * `new_session` - The newly created session.
    /// * `auth` - The token for the newly authenticated principal.
    pub fn on_session_change(
        &self,
        original_session_id: &str,
        new_session: Option<&mut dyn HttpSession>,
        auth: Arc<dyn Authentication>,
    ) {
        if let Some(application_event_publisher) = self.application_event_publisher.as_ref() {
            let event = SessionFixationProtectionEvent::new(
                auth,
                original_session_id,
                new_session.map(|s| s.id()).unwrap_or_default(),
            );
            application_event_publisher.publish_event(Box::new(event));
        }
    }

    pub fn set_application_event_publisher(
        &mut self,
        publisher: Arc<dyn ApplicationEventPublisher>,
    ) {
        self.application_event_publisher = Some(publisher);
    }

    pub fn is_always_create_session(&self) -> bool {
        self.always_create_session
    }

    pub fn set_always_create_session(&mut self, always_create_session: bool) {
        self.always_create_session = always_create_session;
    }
}

#[async_trait]
impl<T> SessionAuthenticationStrategy for T
where
    T: BaseSessionFixationProtectionStrategyExt,
    T: Deref<Target = BaseSessionFixationProtectionStrategy>,
{
    async fn on_authentication(
        &self,
        authentication: &Arc<dyn Authentication>,
        request: &mut dyn HttpRequest,
        _response: &mut dyn HttpResponse,
    ) -> Result<(), AuthenticationError> {
        let had_session_already = request.session().is_some();

        if !had_session_already && !self.is_always_create_session() {
            // Session fixation isn't a problem if there's no session
            return Ok(());
        }

        // Create new session if necessary
        let mut session = request.session_mut(true);
        if had_session_already && request.is_requested_session_id_valid() {
            let original_session_id = session.map(|s| s.id());
            session = self.apply_session_fixation(request);
            let new_session_id = session.map(|s| s.id());

            if original_session_id == new_session_id {
                warn!(
                           "Your servlet container did not change the session ID when a new session \
                            was created. You will not be adequately protected against session-fixation attacks",
                       );
            } else {
                if enabled!(Level::DEBUG) {
                    debug!("Changed session id from {:?}", original_session_id);
                }
            }

            self.on_session_change(
                original_session_id.unwrap_or_default(),
                session,
                authentication.to_owned(),
            );
        }

        Ok(())
    }
}

impl Default for BaseSessionFixationProtectionStrategy {
    fn default() -> Self {
        Self {
            application_event_publisher: None,
            always_create_session: false,
        }
    }
}

pub trait BaseSessionFixationProtectionStrategyExt
where
    Self: Send + Sync,
{
    /// Applies session fixation
    fn apply_session_fixation<'a>(
        &'a self,
        request: &'a mut dyn HttpRequest,
    ) -> Option<&'a mut dyn HttpSession>;
}
