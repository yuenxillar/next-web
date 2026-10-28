//! The service that manages the authorized clients of a principal.

/// Manages the authorized clients of a principal.
pub trait OAuth2AuthorizedClientService: Send + Sync {}

/// An [`OAuth2AuthorizedClientService`] that stores nothing.
#[derive(Clone, Default)]
pub struct NullOAuth2AuthorizedClientService;

impl OAuth2AuthorizedClientService for NullOAuth2AuthorizedClientService {}
