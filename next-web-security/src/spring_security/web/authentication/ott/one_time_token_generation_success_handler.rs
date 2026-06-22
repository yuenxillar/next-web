use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};

use crate::authentication::ott::one_time_token::OneTimeToken;

/// A handler that is invoked when a One-Time Token is successfully generated.
///
/// @author Marcus da Coregio

pub trait OneTimeTokenGenerationSuccessHandler
where
    Self: Send + Sync,
{
    /// Handle the successful generation of a one-time token.
    ///
    /// # Parameters
    /// * `request`  - The HTTP request.
    /// * `response` - The HTTP response.
    /// * `ott`      - The generated One-Time Token.
    fn handle(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        ott: &dyn OneTimeToken,
    );
}
