use std::fmt;

use crate::{
    oauth2::{ClientRegistration, OAuth2AuthorizationExchange},
    web::authentication::Identity,
};

/// Carries the client registration and the authorization exchange through the
/// authentication pipeline. It is stored in the credentials of the token so the
/// authorization code provider can recover it from an `Authentication`.
#[derive(Clone)]
pub(crate) struct OAuth2AuthorizationCodeRequestData {
    pub(crate) client_registration: ClientRegistration,
    pub(crate) authorization_exchange: OAuth2AuthorizationExchange,
}

impl fmt::Display for OAuth2AuthorizationCodeRequestData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "OAuth2AuthorizationCodeRequestData")
    }
}

impl Identity for OAuth2AuthorizationCodeRequestData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
