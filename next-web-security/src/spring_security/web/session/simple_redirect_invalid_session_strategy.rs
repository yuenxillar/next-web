use next_web_core::{
    error::BoxError,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};
use tracing::{enabled, Level};

use crate::web::{
    session::InvalidSessionStrategy, util::UrlUtils, DefaultRedirectStrategy, RedirectStrategy,
};

/// Performs a redirect to a fixed URL when an invalid requested session is detected by
/// the `SessionManagementFilter`.
#[derive(Clone)]
pub struct SimpleRedirectInvalidSessionStrategy {
    destination_url: String,
    redirect_strategy: DefaultRedirectStrategy,
    create_new_session: bool,
}

impl SimpleRedirectInvalidSessionStrategy {
    /// Creates a new instance with the specified redirect URL.
    ///
    /// # Arguments
    ///
    /// * `invalid_session_url` - The URL to redirect to when an invalid session is
    ///   detected. Must start with '/' or with 'http(s)'.
    ///
    /// # Panics
    ///
    /// Panics if the URL is not a valid redirect URL (must start with '/' or with
    /// 'http(s)').
    pub fn new(invalid_session_url: impl Into<String>) -> Self {
        let invalid_session_url = invalid_session_url.into();
        assert!(
            UrlUtils::is_valid_redirect_url(&invalid_session_url),
            "url must start with '/' or with 'http(s)'"
        );
        Self {
            destination_url: invalid_session_url,
            redirect_strategy: DefaultRedirectStrategy::default(),
            create_new_session: true,
        }
    }

    /// Determines whether a new session should be created before redirecting (to avoid
    /// possible looping issues where the same session ID is sent with the redirected
    /// request). Alternatively, ensure that the configured URL does not pass through the
    /// `SessionManagementFilter`.
    ///
    /// # Arguments
    ///
    /// * `create_new_session` - Defaults to `true`.
    pub fn set_create_new_session(&mut self, create_new_session: bool) {
        self.create_new_session = create_new_session;
    }
}

// Implement the InvalidSessionStrategy trait.
impl InvalidSessionStrategy for SimpleRedirectInvalidSessionStrategy {
    fn on_invalid_session_detected(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), BoxError> {
        if enabled!(Level::DEBUG) {
            tracing::debug!(
                "Starting new session (if required) and redirecting to '{}'",
                &self.destination_url
            );
        }
        if self.create_new_session {
            request.session_mut(true);
        }
        self.redirect_strategy
            .send_redirect(request, response, &self.destination_url)?;
        Ok(())
    }
}
