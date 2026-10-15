//! The handler that sends the user to the page a one-time token is submitted
//! on.

use next_web_core::http::{HttpRequest, HttpResponse};

use crate::{
    authentication::ott::one_time_token::OneTimeToken,
    web::{
        authentication::ott::one_time_token_generation_success_handler::OneTimeTokenGenerationSuccessHandler,
        DefaultRedirectStrategy, RedirectStrategy,
    },
};

/// The default path of the page a one-time token is submitted on.
pub const DEFAULT_SUBMIT_PAGE_URL: &str = "/login/ott";

/// Sends the response that was generated for a one-time token to the page the
/// token is submitted on.
///
/// The token is carried as the `token` query parameter of the page, which is
/// what the submit page reads to fill the form in. The page is the one the
/// application configured, and the default one otherwise.
#[derive(Clone, Debug)]
pub struct DefaultOneTimeTokenGenerationSuccessHandler {
    submit_page_url: String,
}

impl DefaultOneTimeTokenGenerationSuccessHandler {
    /// Creates a handler that sends the user to the given page.
    ///
    /// # Arguments
    ///
    /// * `submit_page_url` - The path of the page the token is submitted on.
    pub fn new(submit_page_url: impl Into<String>) -> Self {
        Self {
            submit_page_url: submit_page_url.into(),
        }
    }
}

impl Default for DefaultOneTimeTokenGenerationSuccessHandler {
    fn default() -> Self {
        Self::new(DEFAULT_SUBMIT_PAGE_URL)
    }
}

impl OneTimeTokenGenerationSuccessHandler for DefaultOneTimeTokenGenerationSuccessHandler {
    fn handle(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        ott: &dyn OneTimeToken,
    ) {
        let url = format!(
            "{}?token={}",
            self.submit_page_url,
            urlencoding::encode(ott.token_value())
        );

        if let Err(error) =
            DefaultRedirectStrategy::default().send_redirect(request, response, &url)
        {
            tracing::warn!("the page of the one-time token cannot be reached: {error}");
        }
    }
}
