use std::{any::TypeId, borrow::Cow, sync::Arc};

use next_web_core::error::BoxError;

use crate::{
    core::{
        Authentication, AuthenticationBuilder, GrantedAuthority, Principal, SimpleAuthentication,
    },
    webauthn::AuthenticationRequest,
    web::authentication::AuthPrincipal,
};

/// An authentication request that carries the assertion of a browser.
///
/// The token is what the endpoint converts a submitted ceremony into. It is not
/// authenticated: it only carries the assertion, and the provider decides whether
/// the signature over it holds.
#[derive(Clone, Debug)]
pub struct WebAuthnAuthenticationToken {
    base: SimpleAuthentication,
    request: AuthenticationRequest,
}

impl WebAuthnAuthenticationToken {
    /// Creates a token for a submitted assertion.
    pub fn new(request: AuthenticationRequest) -> Self {
        Self {
            base: SimpleAuthentication::default(),
            request,
        }
    }

    /// Returns the assertion the browser submitted.
    pub fn request(&self) -> &AuthenticationRequest {
        &self.request
    }
}

impl Authentication for WebAuthnAuthenticationToken {
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
        Box::new(WebAuthnAuthenticationTokenBuilder {
            token: self.clone(),
        })
    }

    fn of(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}

/// Rebuilds a [`WebAuthnAuthenticationToken`], so that a provider still sees the
/// assertion after the manager rebuilt the authentication it was handed.
#[derive(Clone, Debug)]
struct WebAuthnAuthenticationTokenBuilder {
    token: WebAuthnAuthenticationToken,
}

impl AuthenticationBuilder for WebAuthnAuthenticationTokenBuilder {
    fn authorities(&mut self, _authorities: Box<dyn FnOnce(&mut Vec<Arc<dyn GrantedAuthority>>)>) {
        // The token grants nothing: the authorities come from the user.
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

impl Principal for WebAuthnAuthenticationToken {
    fn name(&self) -> Cow<'_, str> {
        // The assertion names no principal: the credential it was made with does.
        Cow::Borrowed("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{core::Authentication, webauthn::test_support::authenticate_submission};

    /// Returns a manager and a submission of an assertion of a registered credential.
    fn submission() -> (
        crate::webauthn::WebAuthnManager,
        crate::webauthn::AuthenticationRequest,
    ) {
        let mut manager = crate::webauthn::WebAuthnManager::new(crate::webauthn::RelyingParty::new(
            crate::webauthn::test_support::RP_ID,
            "The application",
        ));
        manager.allowed_origins([crate::webauthn::test_support::ORIGIN]);
        let credentials = Arc::new(crate::webauthn::InMemoryUserCredentialRepository::default());
        manager.credential_repository(credentials);
        crate::webauthn::test_support::register_credential(&manager, b"the-credential", "Max");
        let raw = authenticate_submission(&manager, b"the-credential", "Max", 1);
        let request = crate::webauthn::AuthenticationRequest::parse(&raw)
            .expect("the request parses");

        (manager, request)
    }

    #[test]
    fn the_token_carries_the_assertion() {
        let (_, request) = submission();
        let token = WebAuthnAuthenticationToken::new(request.clone());

        assert_eq!(token.request().credential_id(), b"the-credential");
    }

    #[test]
    fn the_token_is_not_authenticated() {
        let (_, request) = submission();
        let token = WebAuthnAuthenticationToken::new(request);

        assert!(!token.is_authenticated());
        assert!(token.principal().is_none());
    }
}
