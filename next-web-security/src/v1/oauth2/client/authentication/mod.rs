//! The authentication tokens and providers of the OAuth 2.0 client.

mod oauth2_authorization_code_authentication_provider;
mod oauth2_authorization_code_authentication_token;
mod oauth2_authorization_code_request_data;
mod oauth2_login_authentication_provider;
mod oauth2_login_authentication_token;

pub use oauth2_authorization_code_authentication_provider::OAuth2AuthorizationCodeAuthenticationProvider;
pub use oauth2_authorization_code_authentication_token::{
    OAuth2AuthorizationCodeAuthenticationToken, OAuth2AuthorizationCodeAuthenticationTokenBuilder,
};
pub use oauth2_login_authentication_provider::OAuth2LoginAuthenticationProvider;
pub use oauth2_login_authentication_token::{
    OAuth2LoginAuthenticationToken, OAuth2LoginAuthenticationTokenBuilder,
};
