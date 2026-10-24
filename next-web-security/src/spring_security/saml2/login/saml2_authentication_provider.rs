use std::any::{Any, TypeId};
use std::sync::Arc;

use next_web_core::async_trait;

use crate::{
    authentication::AuthenticationProvider,
    core::{userdetails::UserDetailsService, Authentication, AuthenticationError, AuthenticationErrorKind},
    saml2::{
        login::{
            AssertionVerifier, Saml2Authentication, Saml2AuthenticationToken, Saml2Response,
        },
        registration::RelyingPartyRegistrationRepository,
    },
};

/// Authenticates a response the asserting party posted back.
///
/// The provider is the place the checks meet: the response is parsed, the
/// configured [`AssertionVerifier`] decides whether it may be believed, and the
/// name id it carries is loaded as a user of the application. The authorities of
/// that user are what the authentication grants.
pub struct Saml2AuthenticationProvider {
    registrations: Arc<dyn RelyingPartyRegistrationRepository>,
    user_details_service: Arc<dyn UserDetailsService>,
    verifier: Arc<dyn AssertionVerifier>,
}

impl Saml2AuthenticationProvider {
    /// Creates a provider over the given registrations, users and verifier.
    pub fn new(
        registrations: Arc<dyn RelyingPartyRegistrationRepository>,
        user_details_service: Arc<dyn UserDetailsService>,
        verifier: Arc<dyn AssertionVerifier>,
    ) -> Self {
        Self {
            registrations,
            user_details_service,
            verifier,
        }
    }
}

#[async_trait]
impl AuthenticationProvider for Saml2AuthenticationProvider {
    async fn authenticate(
        &self,
        authentication: &Arc<dyn Authentication>,
    ) -> Result<Option<Arc<dyn Authentication>>, AuthenticationError> {
        let Some(token) =
            (authentication.as_ref() as &dyn Any).downcast_ref::<Saml2AuthenticationToken>()
        else {
            return Ok(None);
        };

        let Some(registration) = self
            .registrations
            .find_by_registration_id(token.registration_id())
        else {
            return Err(AuthenticationError::with_kind(
                format!(
                    "the SAML registration {} is not known",
                    token.registration_id()
                ),
                AuthenticationErrorKind::BadCredentials,
            ));
        };

        // The endpoints of a registration are templates; the response was
        // submitted on a request, so its base URL is what they resolve against.
        let registration = match token.base_url() {
            Some(base_url) => registration.resolve(base_url),
            None => registration,
        };

        let response = Saml2Response::parse(token.response_xml()).map_err(|error| {
            AuthenticationError::with_kind(error.message(), AuthenticationErrorKind::BadCredentials)
        })?;

        self.verifier
            .verify(&registration, &response)
            .map_err(|error| {
                AuthenticationError::with_kind(
                    error.message(),
                    AuthenticationErrorKind::BadCredentials,
                )
            })?;

        let Some(name_id) = response.name_id() else {
            return Err(AuthenticationError::with_kind(
                "the SAML response names no subject",
                AuthenticationErrorKind::BadCredentials,
            ));
        };

        let user = self
            .user_details_service
            .load_user_by_username(name_id)
            .await
            .map_err(|_| {
                AuthenticationError::with_kind(
                    format!("the SAML subject {name_id} is not a user of the application"),
                    AuthenticationErrorKind::BadCredentials,
                )
            })?;

        Ok(Some(Arc::new(Saml2Authentication::new(
            name_id,
            user.authorities().to_vec(),
            token.response_xml(),
        ))))
    }

    fn supports(&self, authentication: TypeId) -> bool {
        authentication == TypeId::of::<Saml2AuthenticationToken>()
    }
}

#[cfg(test)]
mod tests {
    use std::any::TypeId;
    use std::sync::Arc;

    use super::*;
    use crate::core::userdetails::{User, UserDetailsService};
    use crate::provisioning::InMemoryUserDetailsManager;
    use crate::saml2::login::{
        AssertionVerifier, RejectingAssertionVerifier, Saml2AuthenticationToken, Saml2Response,
    };
    use crate::saml2::registration::{
        InMemoryRelyingPartyRegistrationRepository, RelyingPartyRegistration,
    };

    /// The response an asserting party posts back for the test registration.
    const RESPONSE: &str = r#"<saml2p:Response xmlns:saml2p="urn:oasis:names:tc:SAML:2.0:protocol" xmlns:saml2="urn:oasis:names:tc:SAML:2.0:assertion"><saml2:Issuer>idp-entity-id</saml2:Issuer><saml2p:Status><saml2p:StatusCode Value="urn:oasis:names:tc:SAML:2.0:status:Success"/></saml2p:Status><saml2:Assertion><saml2:Subject><saml2:NameID>Max</saml2:NameID></saml2:Subject></saml2:Assertion></saml2p:Response>"#;

    /// A verifier that believes every response.
    struct AcceptingVerifier;

    impl AssertionVerifier for AcceptingVerifier {
        fn verify(
            &self,
            _registration: &RelyingPartyRegistration,
            _response: &Saml2Response,
        ) -> Result<(), crate::saml2::Saml2Error> {
            Ok(())
        }
    }

    /// Returns a registration the response is submitted for.
    fn registration() -> RelyingPartyRegistration {
        RelyingPartyRegistration::with_registration_id("okta")
            .entity_id("sp-entity-id")
            .asserting_party_entity_id("idp-entity-id")
            .build()
    }

    /// Returns a provider over one registration and one user.
    fn provider(verifier: Arc<dyn AssertionVerifier>) -> Saml2AuthenticationProvider {
        let user_details_service: Arc<dyn UserDetailsService> =
            Arc::new(InMemoryUserDetailsManager::with_users(vec![Arc::new(
                User::with_username("Max")
                    .password(Some("password".to_string()))
                    .build(),
            )]));

        Saml2AuthenticationProvider::new(
            Arc::new(InMemoryRelyingPartyRegistrationRepository::new(vec![
                registration(),
            ])),
            user_details_service,
            verifier,
        )
    }

    #[test]
    fn the_provider_supports_the_token_the_converter_produces() {
        let provider = provider(Arc::new(AcceptingVerifier));

        assert!(provider.supports(TypeId::of::<Saml2AuthenticationToken>()));
    }

    #[tokio::test]
    async fn a_believed_response_authenticates_the_user_it_names() {
        let provider = provider(Arc::new(AcceptingVerifier));
        let token: Arc<dyn Authentication> =
            Arc::new(Saml2AuthenticationToken::new("okta", RESPONSE, None));

        let authentication = provider
            .authenticate(&token)
            .await
            .expect("the response is believed")
            .expect("the token is a SAML 2.0 token");

        assert!(authentication.is_authenticated());
        assert_eq!(
            authentication
                .principal()
                .map(|principal| principal.to_string()),
            Some("Max".to_owned())
        );
    }

    #[tokio::test]
    async fn a_rejected_response_does_not_authenticate() {
        let provider = provider(Arc::new(RejectingAssertionVerifier));
        let token: Arc<dyn Authentication> =
            Arc::new(Saml2AuthenticationToken::new("okta", RESPONSE, None));

        let error = provider
            .authenticate(&token)
            .await
            .expect_err("no verifier is configured");

        assert!(
            error.to_string().contains("verifier"),
            "the error explains the missing verifier, was: {error}"
        );
    }

    #[tokio::test]
    async fn a_response_for_an_unknown_registration_does_not_authenticate() {
        let provider = provider(Arc::new(AcceptingVerifier));
        let token: Arc<dyn Authentication> =
            Arc::new(Saml2AuthenticationToken::new("azure", RESPONSE, None));

        let error = provider
            .authenticate(&token)
            .await
            .expect_err("the registration is unknown");

        assert!(
            error.to_string().contains("registration"),
            "the error names the registration, was: {error}"
        );
    }

    /// A verifier that expects the registration to be resolved already.
    struct ExpectingEntityId;

    impl AssertionVerifier for ExpectingEntityId {
        fn verify(
            &self,
            registration: &RelyingPartyRegistration,
            _response: &Saml2Response,
        ) -> Result<(), crate::saml2::Saml2Error> {
            assert_eq!(
                registration.entity_id(),
                "https://sp.example.org/saml2/service-provider-metadata/okta",
                "the template is resolved against the request the response came in on"
            );

            Ok(())
        }
    }

    #[tokio::test]
    async fn a_registration_template_is_resolved_before_the_response_is_judged() {
        let user_details_service: Arc<dyn UserDetailsService> =
            Arc::new(InMemoryUserDetailsManager::with_users(vec![Arc::new(
                User::with_username("Max")
                    .password(Some("password".to_string()))
                    .build(),
            )]));
        let registrations = Arc::new(InMemoryRelyingPartyRegistrationRepository::new(vec![
            RelyingPartyRegistration::with_registration_id("okta")
                .asserting_party_entity_id("idp-entity-id")
                .build(),
        ]));
        let provider = Saml2AuthenticationProvider::new(
            registrations,
            user_details_service,
            Arc::new(ExpectingEntityId),
        );
        let token: Arc<dyn Authentication> = Arc::new(
            Saml2AuthenticationToken::new("okta", RESPONSE, None)
                .with_base_url("https://sp.example.org"),
        );

        provider
            .authenticate(&token)
            .await
            .expect("the response is believed")
            .expect("the token is a SAML 2.0 token");
    }
}
