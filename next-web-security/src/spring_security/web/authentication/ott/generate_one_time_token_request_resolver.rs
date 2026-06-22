use next_web_core::traits::http::http_request::HttpRequest;

use crate::authentication::ott::generate_one_time_token_request::GenerateOneTimeTokenRequest;

/// A resolver that extracts a `GenerateOneTimeTokenRequest` from an `HttpRequest`.
pub trait GenerateOneTimeTokenRequestResolver
where
    Self: Send + Sync,
{
    /// Resolves the `GenerateOneTimeTokenRequest` from the request, or returns
    /// `None` if no generate request should be made.
    fn resolve(&self, request: &dyn HttpRequest) -> Option<GenerateOneTimeTokenRequest>;
}

/// Default implementation of `GenerateOneTimeTokenRequestResolver` that extracts
/// the `username` parameter from the request.
///
/// If the `username` parameter is not present or is empty, this resolver
/// returns `None`.
///

#[derive(Clone, Default)]
pub struct DefaultGenerateOneTimeTokenRequestResolver;

impl GenerateOneTimeTokenRequestResolver for DefaultGenerateOneTimeTokenRequestResolver {
    fn resolve(&self, request: &dyn HttpRequest) -> Option<GenerateOneTimeTokenRequest> {
        let username = request.parameter("username")?;
        if username.is_empty() {
            return None;
        }
        Some(GenerateOneTimeTokenRequest::new(username))
    }
}
