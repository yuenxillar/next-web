use super::fallback_error::FallbackError;

pub type FallbackResult = Result<(), ()>;

pub trait FallbackProvider: Send + Sync {
    // Get fallback provider id
    fn id(&self) -> &'static str;

    // Get fallback response for given route and error
    fn fallback_response(&self, route: String, error: FallbackError) -> FallbackResult;
}
