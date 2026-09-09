pub mod bearer;
pub mod entry;
pub mod jwt;
pub mod opaque;

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
