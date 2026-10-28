//! OAuth 2.0 Resource Server support.
//!
//! The module holds the bearer-token support of the resource server, the JWT
//! support the tokens are decoded with, and the opaque-token support that
//! introspects a token instead of decoding it.

pub mod bearer;
pub mod entry;
pub mod jwt;
pub mod opaque;
pub mod web;

pub use bearer::{
    BearerTokenAuthenticationConverter, BearerTokenAuthenticationToken, BearerTokenRequestMatcher,
    BearerTokenResolver, DefaultBearerTokenResolver,
};
pub use entry::{
    AuthenticationEntryPointFailureHandler, BearerTokenAccessDeniedHandler,
    BearerTokenAuthenticationEntryPoint, DPoPConfigurer, OAuth2ProtectedResourceMetadataFilter,
    ProtectedResourceMetadataConfigurer,
};
pub use jwt::{
    DefaultJwtAuthenticationConverter, Jwt, JwtAuthenticationConverter, JwtAuthenticationProvider,
    JwtConfigurer, JwtDecoder, NimbusJwtDecoder,
};
pub use opaque::{
    OAuth2AuthenticatedPrincipal, OAuth2ProtectedResourceMetadata,
    OpaqueTokenAuthenticationConverter, OpaqueTokenAuthenticationProvider, OpaqueTokenConfigurer,
    OpaqueTokenIntrospector, SpringOpaqueTokenIntrospector,
};
pub use web::BearerTokenAuthenticationFilter;
