use std::{any::TypeId, borrow::Cow, sync::Arc};

use next_web_core::error::BoxError;

use crate::{
    core::{
        Authentication, AuthenticationBuilder, GrantedAuthority, Principal, SimpleAuthentication,
    },
    web::authentication::AuthPrincipal,
};

/// An authentication request that carries the response of an asserting party.
///
/// The token is what the assertion consumer endpoint converts a submitted
/// response into. It is not authenticated: it only carries what the asserting
/// party said, and the provider decides whether that is enough.
#[derive(Clone, Debug)]
pub struct Saml2AuthenticationToken {
    base: SimpleAuthentication,
    registration_id: String,
    response_xml: String,
    relay_state: Option<String>,
    base_url: Option<String>,
}

impl Saml2AuthenticationToken {
    /// Creates a token for a submitted response.
    pub fn new(
        registration_id: impl Into<String>,
        response_xml: impl Into<String>,
        relay_state: Option<&str>,
    ) -> Self {
        Self {
            base: SimpleAuthentication::default(),
            registration_id: registration_id.into(),
            response_xml: response_xml.into(),
            relay_state: relay_state.map(str::to_owned),
            base_url: None,
        }
    }

    /// Records the base URL of the request the response was submitted on, so the
    /// templates of the registration can be resolved before it is judged.
    #[must_use]
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    /// Returns the base URL of the request the response was submitted on.
    pub fn base_url(&self) -> Option<&str> {
        self.base_url.as_deref()
    }

    /// Returns the registration the response was submitted to.
    pub fn registration_id(&self) -> &str {
        &self.registration_id
    }

    /// Returns the response document, as it was decoded from the submission.
    pub fn response_xml(&self) -> &str {
        &self.response_xml
    }

    /// Returns the state the asserting party was asked to send back.
    pub fn relay_state(&self) -> Option<&str> {
        self.relay_state.as_deref()
    }
}

impl Authentication for Saml2AuthenticationToken {
    fn authorities(&self) -> &[Arc<dyn GrantedAuthority>] {
        self.base.authorities()
    }

    fn credentials(&self) -> Option<&AuthPrincipal> {
        self.base.credentials()
    }

    fn details(&self) -> Option<&AuthPrincipal> {
        self.base.details()
    }

    fn principal(&self) -> Option<&AuthPrincipal> {
        self.base.principal()
    }

    fn is_authenticated(&self) -> bool {
        self.base.is_authenticated()
    }

    fn set_authenticated(&mut self, is_authenticated: bool) -> Result<(), BoxError> {
        self.base.set_authenticated(is_authenticated)
    }

    fn to_builder(&self) -> Box<dyn AuthenticationBuilder> {
        Box::new(Saml2AuthenticationTokenBuilder {
            token: self.clone(),
        })
    }

    fn of(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}

/// Rebuilds a [`Saml2AuthenticationToken`].
///
/// The authentication manager rebuilds the authentication it is handed before
/// a provider sees it, so the rebuild has to yield the token again — a builder
/// that returned a plain authentication would hide the response from every
/// provider.
#[derive(Clone, Debug)]
struct Saml2AuthenticationTokenBuilder {
    token: Saml2AuthenticationToken,
}

impl AuthenticationBuilder for Saml2AuthenticationTokenBuilder {
    fn authorities(&mut self, _authorities: Box<dyn FnOnce(&mut Vec<Arc<dyn GrantedAuthority>>)>) {
        // The token grants nothing: the authorities are those of the user the
        // provider loads.
    }

    fn credentials(&mut self, _credentials: Option<AuthPrincipal>) {}

    fn details(&mut self, _details: Option<AuthPrincipal>) {}

    fn principal(&mut self, _principal: Option<AuthPrincipal>) {}

    fn authenticated(&mut self, authenticated: bool) {
        let _ = self.token.set_authenticated(authenticated);
    }

    fn build(&mut self) -> Arc<dyn Authentication> {
        Arc::new(self.token.clone())
    }
}

impl Principal for Saml2AuthenticationToken {
    fn name(&self) -> Cow<'_, str> {
        // The token carries no principal yet: the response has not been believed
        // or turned into one.
        Cow::Borrowed("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Authentication;

    #[test]
    fn the_token_carries_the_response_and_the_registration() {
        let token = Saml2AuthenticationToken::new("okta", "<saml2p:Response/>", Some("/start"));

        assert_eq!(token.registration_id(), "okta");
        assert_eq!(token.response_xml(), "<saml2p:Response/>");
        assert_eq!(token.relay_state(), Some("/start"));
    }

    #[test]
    fn the_token_is_not_authenticated() {
        let token = Saml2AuthenticationToken::new("okta", "<saml2p:Response/>", None);

        assert!(!token.is_authenticated());
        assert!(token.principal().is_none());
    }
}
