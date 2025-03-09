use super::fallback_provider::{FallbackProvider, FallbackResult};

pub struct DefaultFallbackProvider;

impl FallbackProvider for DefaultFallbackProvider {
    fn id(&self) -> &'static str {
        "default"
    }

    fn fallback_response(
        &self,
        route: String,
        error: super::fallback_error::FallbackError,
    ) -> FallbackResult {
        todo!()
    }
}
