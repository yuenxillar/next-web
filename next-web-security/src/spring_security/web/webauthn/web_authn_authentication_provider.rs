use std::any::{Any, TypeId};
use std::sync::Arc;

use next_web_core::async_trait;

use crate::{
    authentication::AuthenticationProvider,
    core::{
        userdetails::UserDetailsService, Authentication, AuthenticationError, AuthenticationErrorKind,
    },
    webauthn::WebAuthnManager,
    web::webauthn::{WebAuthnAuthentication, WebAuthnAuthenticationToken},
};

/// Authenticates the assertion a browser submitted.
///
/// The ceremony decides whether the assertion may be believed; the user the
/// credential belongs to is then loaded as a user of the application, and the
/// authorities of that user are what the authentication grants. A credential of
/// a user the application does not have authenticates nobody.
pub struct WebAuthnAuthenticationProvider {
    manager: Arc<WebAuthnManager>,
    user_details_service: Arc<dyn UserDetailsService>,
}

impl WebAuthnAuthenticationProvider {
    /// Creates a provider over the given ceremonies and users.
    pub fn new(
        manager: Arc<WebAuthnManager>,
        user_details_service: Arc<dyn UserDetailsService>,
    ) -> Self {
        Self {
            manager,
            user_details_service,
        }
    }
}

#[async_trait]
impl AuthenticationProvider for WebAuthnAuthenticationProvider {
    async fn authenticate(
        &self,
        authentication: &Arc<dyn Authentication>,
    ) -> Result<Option<Arc<dyn Authentication>>, AuthenticationError> {
        let Some(token) =
            (authentication.as_ref() as &dyn Any).downcast_ref::<WebAuthnAuthenticationToken>()
        else {
            return Ok(None);
        };

        let credential = self.manager.authenticate(token.request()).map_err(|error| {
            AuthenticationError::with_kind(error.message(), AuthenticationErrorKind::BadCredentials)
        })?;

        let user = self
            .user_details_service
            .load_user_by_username(credential.user_name())
            .await
            .map_err(|_| {
                AuthenticationError::with_kind(
                    format!(
                        "the credential authenticates {}, who is not a user of the application",
                        credential.user_name()
                    ),
                    AuthenticationErrorKind::BadCredentials,
                )
            })?;

        Ok(Some(Arc::new(WebAuthnAuthentication::new(
            credential.user_name(),
            user.authorities().to_vec(),
            credential.credential_id().to_vec(),
        ))))
    }

    fn supports(&self, authentication: TypeId) -> bool {
        authentication == TypeId::of::<WebAuthnAuthenticationToken>()
    }
}

#[cfg(test)]
mod tests {
    use std::any::TypeId;

    use super::*;
    use crate::{
        core::userdetails::{User, UserDetailsService},
        provisioning::InMemoryUserDetailsManager,
        webauthn::{
            test_support::{authenticate_submission, register_credential, ORIGIN, RP_ID},
            InMemoryUserCredentialRepository, RelyingParty, WebAuthnManager,
        },
        web::webauthn::WebAuthnAuthenticationToken,
    };

    /// Returns a manager that registered `the-credential` for `Max`.
    fn manager() -> Arc<WebAuthnManager> {
        let credentials = Arc::new(InMemoryUserCredentialRepository::default());
        let mut manager = WebAuthnManager::new(RelyingParty::new(RP_ID, "The application"));
        manager.allowed_origins([ORIGIN]);
        manager.credential_repository(credentials);
        register_credential(&manager, b"the-credential", "Max");

        Arc::new(manager)
    }

    /// Returns a user details service that holds `Max`.
    fn user_details_service() -> Arc<dyn UserDetailsService> {
        Arc::new(InMemoryUserDetailsManager::with_users(vec![Arc::new(
            User::with_username("Max")
                .password(Some("password".to_string()))
                .build(),
        )]))
    }

    /// Returns the token of an assertion of the registered credential.
    fn token(manager: &Arc<WebAuthnManager>) -> Arc<dyn Authentication> {
        let raw = authenticate_submission(manager, b"the-credential", "Max", 1);
        let request = crate::webauthn::AuthenticationRequest::parse(&raw).expect("the request parses");

        Arc::new(WebAuthnAuthenticationToken::new(request))
    }

    #[test]
    fn the_provider_supports_the_token_the_converter_produces() {
        let provider =
            WebAuthnAuthenticationProvider::new(manager(), user_details_service());

        assert!(provider.supports(TypeId::of::<WebAuthnAuthenticationToken>()));
    }

    #[tokio::test]
    async fn a_verified_assertion_authenticates_the_user_of_the_credential() {
        let manager = manager();
        let provider = WebAuthnAuthenticationProvider::new(manager.clone(), user_details_service());

        let authentication = provider
            .authenticate(&token(&manager))
            .await
            .expect("the assertion verifies")
            .expect("the token is a WebAuthn token");

        assert!(authentication.is_authenticated());
        assert_eq!(
            authentication.principal().map(|principal| principal.to_string()),
            Some("Max".to_owned())
        );
    }

    #[tokio::test]
    async fn a_replayed_assertion_is_refused() {
        let manager = manager();
        let provider = WebAuthnAuthenticationProvider::new(manager.clone(), user_details_service());
        let token = token(&manager);

        provider
            .authenticate(&token)
            .await
            .expect("the first assertion holds");
        let error = provider
            .authenticate(&token)
            .await
            .expect_err("the challenge was answered already");

        assert!(error.to_string().contains("challenge"), "was: {error}");
    }

    #[tokio::test]
    async fn an_assertion_of_a_user_the_application_does_not_have_is_refused() {
        let credentials = Arc::new(InMemoryUserCredentialRepository::default());
        let mut manager = WebAuthnManager::new(RelyingParty::new(RP_ID, "The application"));
        manager.allowed_origins([ORIGIN]);
        manager.credential_repository(credentials);
        register_credential(&manager, b"the-credential", "Bob");
        let manager = Arc::new(manager);
        let provider = WebAuthnAuthenticationProvider::new(manager.clone(), user_details_service());

        let error = provider
            .authenticate(&token(&manager))
            .await
            .expect_err("Bob is not a user of the application");

        assert!(error.to_string().contains("not a user"), "was: {error}");
    }
}
