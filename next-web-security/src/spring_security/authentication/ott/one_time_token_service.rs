use super::{
    default_one_time_token::DefaultOneTimeToken,
    generate_one_time_token_request::GenerateOneTimeTokenRequest,
    one_time_token_authentication_token::OneTimeTokenAuthenticationToken,
};

pub trait OneTimeTokenService: Send + Sync {
    fn generate(&self, request: GenerateOneTimeTokenRequest) -> DefaultOneTimeToken;

    fn consume(
        &self,
        authentication_token: &OneTimeTokenAuthenticationToken,
    ) -> Option<DefaultOneTimeToken>;
}
