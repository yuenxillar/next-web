use std::sync::Arc;

use super::{
    generate_one_time_token_request::GenerateOneTimeTokenRequest, one_time_token::OneTimeToken,
    one_time_token_authentication_token::OneTimeTokenAuthenticationToken,
};

pub trait OneTimeTokenService: Send + Sync {
    fn generate(&self, request: GenerateOneTimeTokenRequest) -> Arc<dyn OneTimeToken>;

    fn consume(
        &self,
        authentication_token: &OneTimeTokenAuthenticationToken,
    ) -> Option<Arc<dyn OneTimeToken>>;
}
