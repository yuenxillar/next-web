use std::sync::Arc;

use next_web_core::error::BoxError;
use tracing::debug;

use crate::web::{
    session::{SessionInformationExpiredEvent, SessionInformationExpiredStrategy},
    util::UrlUtils,
    DefaultRedirectStrategy, RedirectStrategy,
};

/// Performs a redirect to a fixed URL when an expired session is detected by the
/// `ConcurrentSessionFilter`.
pub struct SimpleRedirectSessionInformationExpiredStrategy {
    destination_url: String,
    redirect_strategy: Arc<dyn RedirectStrategy>,
}

impl SimpleRedirectSessionInformationExpiredStrategy {
    /// Creates a new instance with the specified redirect URL and a default
    /// `DefaultRedirectStrategy`.
    ///
    /// # Arguments
    ///
    /// * `invalid_session_url` - The URL to redirect to when an expired session is
    ///   detected. Must start with '/' or with 'http(s)'.
    ///
    /// # Panics
    ///
    /// Panics if the URL is not a valid redirect URL (must start with '/' or with
    /// 'http(s)').
    pub fn new(invalid_session_url: impl Into<String>) -> Self {
        Self::with_redirect_strategy(
            invalid_session_url,
            Arc::new(DefaultRedirectStrategy::default()),
        )
    }

    /// Creates a new instance with the specified redirect URL and a custom
    /// `RedirectStrategy`.
    ///
    /// # Arguments
    ///
    /// * `invalid_session_url` - The URL to redirect to when an expired session is
    ///   detected. Must start with '/' or with 'http(s)'.
    /// * `redirect_strategy` - The `RedirectStrategy` to use for performing the
    ///   redirect.
    ///
    /// # Panics
    ///
    /// Panics if the URL is not a valid redirect URL (must start with '/' or with
    /// 'http(s)').
    pub fn with_redirect_strategy(
        invalid_session_url: impl Into<String>,
        redirect_strategy: Arc<dyn RedirectStrategy>,
    ) -> Self {
        let invalid_session_url = invalid_session_url.into();
        assert!(
            UrlUtils::is_valid_redirect_url(&invalid_session_url),
            "url must start with '/' or with 'http(s)'"
        );
        Self {
            destination_url: invalid_session_url,
            redirect_strategy,
        }
    }
}

// Implement the SessionInformationExpiredStrategy trait.
impl SessionInformationExpiredStrategy for SimpleRedirectSessionInformationExpiredStrategy {
    fn on_expired_session_detected(
        &self,
        event: &mut SessionInformationExpiredEvent,
    ) -> Result<(), BoxError> {
        debug!("Redirecting to '{}'", &self.destination_url);

        let (req, resp) = event.parts_mut();
        self.redirect_strategy
            .send_redirect(req, resp, &self.destination_url)
    }
}
