use super::default_token::DefaultToken;

pub trait TokenService: Send + Sync {
    fn allocate_token(&self, extended_information: impl Into<String>) -> DefaultToken;

    fn verify_token(&self, key: &str) -> Option<DefaultToken>;
}
