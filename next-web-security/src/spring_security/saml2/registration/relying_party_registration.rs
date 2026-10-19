use crate::saml2::registration::Saml2MessageBinding;

/// The template the entity id of a registration is derived from when none is
/// given.
///
/// The service provider metadata URL carries the registration id, so an
/// asserting party that fetches the metadata finds the same identifier there.
pub const DEFAULT_ENTITY_ID_TEMPLATE: &str =
    "{baseUrl}/saml2/service-provider-metadata/{registrationId}";

/// The template the assertion consumer service location is derived from when
/// none is given.
pub const DEFAULT_ASSERTION_CONSUMER_SERVICE_LOCATION_TEMPLATE: &str =
    "{baseUrl}/login/saml2/sso/{registrationId}";

/// One relying party the service provider authenticates with.
///
/// The endpoints are templates: `{baseUrl}` is replaced by the base URL of the
/// request that reads them, and `{registrationId}` by the registration id, so a
/// registration does not have to know which host it is deployed on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelyingPartyRegistration {
    registration_id: String,
    entity_id: String,
    assertion_consumer_service_location: String,
    assertion_consumer_service_binding: Saml2MessageBinding,
    single_logout_service_location: Option<String>,
    single_logout_service_response_location: Option<String>,
    single_logout_service_binding: Saml2MessageBinding,
    name_id_format: Option<String>,
    signing_certificate: Option<String>,
    encryption_certificate: Option<String>,
    asserting_party_entity_id: Option<String>,
    single_sign_on_service_location: Option<String>,
    single_sign_on_service_binding: Saml2MessageBinding,
    verification_public_key: Option<String>,
}

impl RelyingPartyRegistration {
    /// Starts a registration with the given identifier.
    ///
    /// # Panics
    ///
    /// Panics when the identifier is empty or whitespace, because it names the
    /// registration in every URL that carries one.
    pub fn with_registration_id(
        registration_id: impl Into<String>,
    ) -> RelyingPartyRegistrationBuilder {
        let registration_id = registration_id.into();
        assert!(
            !registration_id.trim().is_empty(),
            "registrationId cannot be null or empty"
        );

        RelyingPartyRegistrationBuilder::new(registration_id)
    }

    /// Returns the identifier this registration is looked up by.
    pub fn registration_id(&self) -> &str {
        &self.registration_id
    }

    /// Returns the identifier the two sides use for the service provider.
    pub fn entity_id(&self) -> &str {
        &self.entity_id
    }

    /// Returns the location the asserting party posts an assertion to.
    pub fn assertion_consumer_service_location(&self) -> &str {
        &self.assertion_consumer_service_location
    }

    /// Returns the binding the asserting party posts an assertion with.
    pub fn assertion_consumer_service_binding(&self) -> Saml2MessageBinding {
        self.assertion_consumer_service_binding
    }

    /// Returns the location the asserting party sends a logout request to, if
    /// the registration supports single logout.
    pub fn single_logout_service_location(&self) -> Option<&str> {
        self.single_logout_service_location.as_deref()
    }

    /// Returns the location the asserting party sends a logout response to.
    pub fn single_logout_service_response_location(&self) -> Option<&str> {
        self.single_logout_service_response_location.as_deref()
    }

    /// Returns the binding the asserting party exchanges logout messages over.
    pub fn single_logout_service_binding(&self) -> Saml2MessageBinding {
        self.single_logout_service_binding
    }

    /// Returns the name id format the asserting party is asked for, if any.
    pub fn name_id_format(&self) -> Option<&str> {
        self.name_id_format.as_deref()
    }

    /// Returns the signing certificate, in PEM form, if the registration has one.
    pub fn signing_certificate(&self) -> Option<&str> {
        self.signing_certificate.as_deref()
    }

    /// Returns the encryption certificate, in PEM form, if the registration has
    /// one.
    pub fn encryption_certificate(&self) -> Option<&str> {
        self.encryption_certificate.as_deref()
    }

    /// Returns the identifier the asserting party calls itself by.
    pub fn asserting_party_entity_id(&self) -> Option<&str> {
        self.asserting_party_entity_id.as_deref()
    }

    /// Returns the location an authentication request is sent to.
    pub fn single_sign_on_service_location(&self) -> Option<&str> {
        self.single_sign_on_service_location.as_deref()
    }

    /// Returns the binding an authentication request is sent with.
    pub fn single_sign_on_service_binding(&self) -> Saml2MessageBinding {
        self.single_sign_on_service_binding
    }

    /// Returns the key the asserting party signs its responses with, in PEM
    /// form, if the registration has one.
    ///
    /// The key is what a response is verified against. It is held as a public key
    /// rather than as a certificate because the port has no X.509 parser: the
    /// certificate body is the same key, wrapped.
    pub fn verification_public_key(&self) -> Option<&str> {
        self.verification_public_key.as_deref()
    }

    /// Returns the registration with its placeholders filled in for one request.
    ///
    /// The endpoints of a registration are templates: `{baseUrl}` is the base URL
    /// of the request, and `{registrationId}` the registration id. A value that
    /// carries no placeholder is returned unchanged, so a registration may name
    /// absolute endpoints instead.
    pub fn resolve(&self, base_url: &str) -> Self {
        let resolve = |value: &str| crate::saml2::xml::resolve_placeholders(value, self, base_url);

        Self {
            entity_id: resolve(&self.entity_id),
            assertion_consumer_service_location: resolve(&self.assertion_consumer_service_location),
            single_logout_service_location: self
                .single_logout_service_location
                .as_deref()
                .map(resolve),
            single_logout_service_response_location: self
                .single_logout_service_response_location
                .as_deref()
                .map(resolve),
            ..self.clone()
        }
    }
}

impl Default for RelyingPartyRegistration {
    fn default() -> Self {
        Self {
            registration_id: String::new(),
            entity_id: DEFAULT_ENTITY_ID_TEMPLATE.to_string(),
            assertion_consumer_service_location:
                DEFAULT_ASSERTION_CONSUMER_SERVICE_LOCATION_TEMPLATE.to_string(),
            assertion_consumer_service_binding: Saml2MessageBinding::Post,
            single_logout_service_location: None,
            single_logout_service_response_location: None,
            single_logout_service_binding: Saml2MessageBinding::Post,
            name_id_format: None,
            signing_certificate: None,
            encryption_certificate: None,
            asserting_party_entity_id: None,
            single_sign_on_service_location: None,
            single_sign_on_service_binding: Saml2MessageBinding::Redirect,
            verification_public_key: None,
        }
    }
}

/// Builds a [`RelyingPartyRegistration`], one optional value at a time.
#[derive(Clone, Debug)]
pub struct RelyingPartyRegistrationBuilder {
    registration: RelyingPartyRegistration,
}

impl RelyingPartyRegistrationBuilder {
    /// Starts a builder for a registration with the given identifier.
    fn new(registration_id: String) -> Self {
        let mut registration = RelyingPartyRegistration::default();
        registration.registration_id = registration_id;

        Self { registration }
    }

    /// Sets the identifier the two sides use for the service provider.
    #[must_use]
    pub fn entity_id(mut self, entity_id: impl Into<String>) -> Self {
        self.registration.entity_id = entity_id.into();
        self
    }

    /// Sets the location the asserting party posts an assertion to.
    #[must_use]
    pub fn assertion_consumer_service_location(mut self, location: impl Into<String>) -> Self {
        self.registration.assertion_consumer_service_location = location.into();
        self
    }

    /// Sets the binding the asserting party posts an assertion with.
    #[must_use]
    pub fn assertion_consumer_service_binding(mut self, binding: Saml2MessageBinding) -> Self {
        self.registration.assertion_consumer_service_binding = binding;
        self
    }

    /// Sets the location the asserting party sends a logout request to.
    ///
    /// A registration without a location publishes no logout endpoint, and the
    /// response location defaults to this one.
    #[must_use]
    pub fn single_logout_service_location(mut self, location: impl Into<String>) -> Self {
        self.registration.single_logout_service_location = Some(location.into());
        self
    }

    /// Sets the location the asserting party sends a logout response to.
    #[must_use]
    pub fn single_logout_service_response_location(
        mut self,
        location: impl Into<String>,
    ) -> Self {
        self.registration.single_logout_service_response_location = Some(location.into());
        self
    }

    /// Sets the binding the asserting party exchanges logout messages over.
    #[must_use]
    pub fn single_logout_service_binding(mut self, binding: Saml2MessageBinding) -> Self {
        self.registration.single_logout_service_binding = binding;
        self
    }

    /// Sets the name id format the asserting party is asked for.
    #[must_use]
    pub fn name_id_format(mut self, name_id_format: impl Into<String>) -> Self {
        self.registration.name_id_format = Some(name_id_format.into());
        self
    }

    /// Sets the certificate the service provider signs its messages with, in PEM
    /// form.
    #[must_use]
    pub fn signing_certificate(mut self, certificate: impl Into<String>) -> Self {
        self.registration.signing_certificate = Some(certificate.into());
        self
    }

    /// Sets the certificate the service provider is sent encrypted assertions
    /// with, in PEM form.
    #[must_use]
    pub fn encryption_certificate(mut self, certificate: impl Into<String>) -> Self {
        self.registration.encryption_certificate = Some(certificate.into());
        self
    }

    /// Sets the identifier the asserting party calls itself by.
    #[must_use]
    pub fn asserting_party_entity_id(mut self, entity_id: impl Into<String>) -> Self {
        self.registration.asserting_party_entity_id = Some(entity_id.into());
        self
    }

    /// Sets the location an authentication request is sent to.
    #[must_use]
    pub fn single_sign_on_service_location(mut self, location: impl Into<String>) -> Self {
        self.registration.single_sign_on_service_location = Some(location.into());
        self
    }

    /// Sets the binding an authentication request is sent with.
    #[must_use]
    pub fn single_sign_on_service_binding(mut self, binding: Saml2MessageBinding) -> Self {
        self.registration.single_sign_on_service_binding = binding;
        self
    }

    /// Sets the key the asserting party signs its responses with, in PEM form.
    #[must_use]
    pub fn verification_public_key(mut self, public_key: impl Into<String>) -> Self {
        self.registration.verification_public_key = Some(public_key.into());
        self
    }

    /// Returns the registration the builder describes.
    ///
    /// A logout response location that was not set is the logout location, so a
    /// registration that publishes a logout endpoint publishes where the
    /// response goes as well.
    pub fn build(mut self) -> RelyingPartyRegistration {
        if self.registration.single_logout_service_response_location.is_none() {
            self.registration.single_logout_service_response_location =
                self.registration.single_logout_service_location.clone();
        }

        self.registration
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_entity_id_defaults_to_the_metadata_url_of_the_registration() {
        let registration = RelyingPartyRegistration::with_registration_id("okta").build();

        assert_eq!(
            registration.entity_id(),
            "{baseUrl}/saml2/service-provider-metadata/{registrationId}"
        );
    }

    #[test]
    fn the_service_locations_default_to_the_endpoints_of_the_registration() {
        let registration = RelyingPartyRegistration::with_registration_id("okta").build();

        assert_eq!(
            registration.assertion_consumer_service_location(),
            "{baseUrl}/login/saml2/sso/{registrationId}"
        );
    }

    #[test]
    fn a_value_given_to_the_builder_replaces_the_default() {
        let registration = RelyingPartyRegistration::with_registration_id("okta")
            .entity_id("https://sp.example.org/saml2")
            .assertion_consumer_service_location("https://sp.example.org/acs")
            .single_logout_service_location("https://sp.example.org/slo")
            .signing_certificate("-----BEGIN CERTIFICATE-----")
            .build();

        assert_eq!(registration.entity_id(), "https://sp.example.org/saml2");
        assert_eq!(
            registration.assertion_consumer_service_location(),
            "https://sp.example.org/acs"
        );
        assert_eq!(
            registration.single_logout_service_location(),
            Some("https://sp.example.org/slo")
        );
        assert_eq!(
            registration.signing_certificate(),
            Some("-----BEGIN CERTIFICATE-----")
        );
    }

    #[test]
    fn a_registration_without_logout_has_no_logout_location() {
        let registration = RelyingPartyRegistration::with_registration_id("okta").build();

        assert_eq!(registration.single_logout_service_location(), None);
        assert_eq!(registration.single_logout_service_response_location(), None);
    }

    #[test]
    fn the_logout_response_location_defaults_to_the_logout_location() {
        let registration = RelyingPartyRegistration::with_registration_id("okta")
            .single_logout_service_location("https://sp.example.org/logout/saml2/slo")
            .build();

        assert_eq!(
            registration.single_logout_service_location(),
            Some("https://sp.example.org/logout/saml2/slo")
        );
        assert_eq!(
            registration.single_logout_service_response_location(),
            Some("https://sp.example.org/logout/saml2/slo")
        );
    }

    #[test]
    fn the_bindings_default_to_http_post() {
        let registration = RelyingPartyRegistration::with_registration_id("okta").build();

        assert_eq!(
            registration.assertion_consumer_service_binding().urn(),
            "urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST"
        );
        assert_eq!(
            registration.single_logout_service_binding().urn(),
            "urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST"
        );
    }

    #[test]
    fn a_binding_given_to_the_builder_replaces_the_default() {
        let registration = RelyingPartyRegistration::with_registration_id("okta")
            .assertion_consumer_service_binding(Saml2MessageBinding::Redirect)
            .single_logout_service_binding(Saml2MessageBinding::Redirect)
            .build();

        assert_eq!(
            registration.assertion_consumer_service_binding().urn(),
            "urn:oasis:names:tc:SAML:2.0:bindings:HTTP-Redirect"
        );
        assert_eq!(
            registration.single_logout_service_binding().urn(),
            "urn:oasis:names:tc:SAML:2.0:bindings:HTTP-Redirect"
        );
    }

    #[test]
    fn a_registration_has_no_name_id_format_until_one_is_set() {
        let registration = RelyingPartyRegistration::with_registration_id("okta")
            .name_id_format("urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress")
            .build();

        assert_eq!(
            registration.name_id_format(),
            Some("urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress")
        );
        assert_eq!(
            RelyingPartyRegistration::with_registration_id("okta")
                .build()
                .name_id_format(),
            None
        );
    }

    #[test]
    fn the_asserting_party_defaults_to_an_unconfigured_identity_provider() {
        let registration = RelyingPartyRegistration::with_registration_id("okta").build();

        assert_eq!(registration.asserting_party_entity_id(), None);
        assert_eq!(registration.single_sign_on_service_location(), None);
        assert_eq!(registration.verification_public_key(), None);
        assert_eq!(
            registration.single_sign_on_service_binding(),
            Saml2MessageBinding::Redirect
        );
    }

    #[test]
    fn the_asserting_party_values_given_to_the_builder_are_kept() {
        let registration = RelyingPartyRegistration::with_registration_id("okta")
            .asserting_party_entity_id("idp-entity-id")
            .single_sign_on_service_location("https://idp.example.org/sso")
            .single_sign_on_service_binding(Saml2MessageBinding::Post)
            .verification_public_key("-----BEGIN PUBLIC KEY-----")
            .build();

        assert_eq!(
            registration.asserting_party_entity_id(),
            Some("idp-entity-id")
        );
        assert_eq!(
            registration.single_sign_on_service_location(),
            Some("https://idp.example.org/sso")
        );
        assert_eq!(
            registration.single_sign_on_service_binding(),
            Saml2MessageBinding::Post
        );
        assert_eq!(
            registration.verification_public_key(),
            Some("-----BEGIN PUBLIC KEY-----")
        );
    }

    #[test]
    fn the_registration_id_is_kept() {
        let registration = RelyingPartyRegistration::with_registration_id("okta").build();

        assert_eq!(registration.registration_id(), "okta");
    }

    #[test]
    #[should_panic(expected = "registrationId cannot be null or empty")]
    fn an_empty_registration_id_is_rejected() {
        RelyingPartyRegistration::with_registration_id(" ");
    }
}
