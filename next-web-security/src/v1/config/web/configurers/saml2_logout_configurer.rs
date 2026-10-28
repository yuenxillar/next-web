use std::ops::{Deref, DerefMut};
use std::sync::{Arc, RwLock};

use next_web_core::{ApplicationContext, Required};

use crate::{
    config::{
        security_builder::SecurityBuilder,
        security_configurer::SecurityConfigurer,
        security_configurer_adapter::SecurityConfigurerAdapter,
        web::{configurers::BaseHttpConfigurer, http_security_builder::HttpSecurityBuilder},
    },
    saml2::registration::RelyingPartyRegistrationRepository,
    web::{
        authentication::logout::{LogoutHandler, SecurityContextLogoutHandler},
        csrf::CsrfFilter,
        default_security_filter_chain::DefaultSecurityFilterChain,
        saml2::logout::{Saml2LogoutRequestFilter, Saml2LogoutResponseFilter},
    },
};

/// Configures SAML 2.0 Single Logout (SLO).
///
/// The configurer serves the single logout endpoint of the registrations: a
/// `LogoutRequest` the asserting party sends there is verified against the key of
/// the registration, ends the session through the configured logout handlers, and
/// is answered with a `LogoutResponse` in the location the registration names; a
/// `LogoutResponse` the asserting party sends there completes a logout of the
/// application and sends the user to the success URL.
///
/// The logout handlers are the ones of the [`LogoutConfigurer`] when the
/// application configures one, so a logout ends the session the same way whether
/// it is started at the application or at the asserting party. Without one, the
/// session is ended by the [`SecurityContextLogoutHandler`].
///
/// Requires a [`RelyingPartyRegistrationRepository`] among the shared objects.
///
/// [`LogoutConfigurer`]: crate::config::web::configurers::LogoutConfigurer
#[derive(Clone)]
pub struct Saml2LogoutConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    logout_url: Option<String>,
    logout_success_url: Option<String>,
    relying_party_registration_repository: Option<Arc<dyn RelyingPartyRegistrationRepository>>,
    logout_handlers: Option<Vec<Arc<dyn LogoutHandler>>>,

    base: BaseHttpConfigurer<Self, H>,
}

impl<H> Saml2LogoutConfigurer<H>
where
    H: HttpSecurityBuilder<H> + 'static,
{
    pub fn new(_context: Arc<RwLock<Box<dyn ApplicationContext>>>) -> Self {
        Self::default()
    }

    /// Sets the path the single logout endpoint of the asserting party answers on.
    ///
    /// The endpoint is the one the registration names as its single logout
    /// service location; the value is kept for the applications that serve a
    /// registration whose location is the default one.
    pub fn logout_url(&mut self, url: &str) -> &mut Self {
        self.logout_url = Some(url.to_string());
        self
    }

    /// Sets the URL the user is sent to once a logout is complete.
    pub fn logout_success_url(&mut self, url: &str) -> &mut Self {
        self.logout_success_url = Some(url.to_string());
        self
    }

    /// Sets the repository of the relying party registrations.
    pub fn relying_party_registration_repository(
        &mut self,
        repository: Arc<dyn RelyingPartyRegistrationRepository>,
    ) -> &mut Self {
        self.relying_party_registration_repository = Some(repository);
        self
    }

    /// Sets the handlers that end the session of a logout.
    pub fn logout_handlers(&mut self, logout_handlers: Vec<Arc<dyn LogoutHandler>>) -> &mut Self {
        self.logout_handlers = Some(logout_handlers);
        self
    }

    /// Returns the handlers that end the session of a logout.
    fn logout_handlers_of(&self, http: &H) -> Vec<Arc<dyn LogoutHandler>> {
        if let Some(logout_handlers) = &self.logout_handlers {
            return logout_handlers.clone();
        }
        if let Some(logout) =
            http.configurer::<crate::config::web::configurers::LogoutConfigurer<H>>()
        {
            let handlers = logout.get_logout_handlers().to_vec();
            if !handlers.is_empty() {
                return handlers;
            }
        }

        vec![Arc::new(SecurityContextLogoutHandler::default())]
    }

    /// Returns the registrations the logout endpoint serves.
    fn registrations(&self, http: &H) -> Arc<dyn RelyingPartyRegistrationRepository> {
        self.relying_party_registration_repository
            .clone()
            .or_else(|| {
                http.shared_object::<Arc<dyn RelyingPartyRegistrationRepository>>()
                    .cloned()
            })
            .expect("a RelyingPartyRegistrationRepository is required to enable saml2Logout()")
    }

    /// Returns the path the logout endpoint listens on.
    fn logout_url_of(&self) -> &str {
        self.logout_url
            .as_deref()
            .unwrap_or(crate::web::saml2::logout::DEFAULT_LOGOUT_REQUEST_URL)
    }

    /// Returns the URL the user is sent to once a logout is complete.
    fn logout_success_url_of(&self) -> &str {
        self.logout_success_url
            .as_deref()
            .unwrap_or("/login?logout")
    }
}

impl<H> Default for Saml2LogoutConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn default() -> Self {
        Self {
            logout_url: None,
            logout_success_url: None,
            relying_party_registration_repository: None,
            logout_handlers: None,
            base: Default::default(),
        }
    }
}

impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>>
    for Saml2LogoutConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: SecurityBuilder<DefaultSecurityFilterChain>,
{
    fn get_object(&self) -> &SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base.get_object()
    }

    fn get_mut_object(&mut self) -> &mut SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base.get_mut_object()
    }
}

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for Saml2LogoutConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: SecurityBuilder<DefaultSecurityFilterChain>,
    H: 'static,
{
    fn init(&mut self, http: &mut H) {
        // The registrations are read while initializing, so a configuration that is
        // missing them is reported before a request reaches the endpoint.
        let registrations = self.registrations(http);
        http.set_shared_object::<Arc<dyn RelyingPartyRegistrationRepository>>(registrations);
    }

    fn configure(&mut self, http: &mut H) {
        let registrations = self.registrations(http);
        let logout_handlers = Arc::new(self.logout_handlers_of(http));

        let mut logout_request =
            Saml2LogoutRequestFilter::new(registrations.clone(), logout_handlers.clone());
        logout_request.set_logout_request_url(self.logout_url_of());
        logout_request.set_logout_success_url(self.logout_success_url_of());
        http.add_filter_before::<Saml2LogoutRequestFilter, CsrfFilter>(logout_request);

        let mut logout_response = Saml2LogoutResponseFilter::new(registrations, logout_handlers);
        logout_response.set_logout_response_url(self.logout_url_of());
        logout_response.set_logout_success_url(self.logout_success_url_of());
        http.add_filter_before::<Saml2LogoutResponseFilter, CsrfFilter>(logout_response);
    }
}

impl<H> Deref for Saml2LogoutConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    type Target = BaseHttpConfigurer<Self, H>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<H> DerefMut for Saml2LogoutConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use axum::{body::Body, extract::Request, response::Response};
    use base64::{engine::general_purpose::STANDARD, Engine};

    use crate::{
        config::{
            authentication::builders::authentication_manager_builder::AuthenticationManagerBuilder,
            security_builder::SecurityBuilder, web::builders::HttpSecurity,
        },
        core::context::SecurityContextHolder,
        saml2::registration::{
            InMemoryRelyingPartyRegistrationRepository, RelyingPartyRegistration,
        },
        web::{security_filter_chain::SecurityFilterChain, test_support::Chain},
    };

    /// The location the asserting party sends its logout request to.
    const SP_LOGOUT_LOCATION: &str = "https://sp.example.org/logout/saml2/slo";
    /// The location the asserting party is answered on.
    const IDP_LOGOUT_LOCATION: &str = "https://idp.example.org/slo";

    /// Returns a key the signed messages of the tests are signed with.
    fn signing_key() -> rsa::RsaPrivateKey {
        rsa::RsaPrivateKey::new(&mut rsa::rand_core::OsRng, 2048).expect("a key is generated")
    }

    /// Returns the public key of a private key, in PEM form.
    fn public_key_pem(key: &rsa::RsaPrivateKey) -> String {
        use rsa::pkcs8::{EncodePublicKey, LineEnding};

        rsa::RsaPublicKey::from(key)
            .to_public_key_pem(LineEnding::LF)
            .expect("the key is written")
    }

    /// Returns the registration the logout of the tests is served for.
    fn registration(key: &rsa::RsaPrivateKey) -> RelyingPartyRegistration {
        RelyingPartyRegistration::with_registration_id("okta")
            .entity_id("sp-entity-id")
            .assertion_consumer_service_location("https://sp.example.org/login/saml2/sso/okta")
            .asserting_party_entity_id("https://idp.example.org")
            .single_sign_on_service_location("https://idp.example.org/sso")
            .single_logout_service_location(SP_LOGOUT_LOCATION)
            .single_logout_service_response_location(IDP_LOGOUT_LOCATION)
            .verification_public_key(public_key_pem(key))
            .build()
    }

    /// Returns the signature element of a message, over the message without it.
    fn signature_of(message: &str, id: &str, key: &rsa::RsaPrivateKey) -> String {
        use rsa::{
            pkcs1v15::SigningKey,
            signature::{SignatureEncoding, Signer},
        };
        use sha2::{Digest, Sha256};

        let digest = STANDARD.encode(Sha256::digest(message.as_bytes()));
        let signature_element_of = |digest: &str, signature: &str| {
            format!(
                r##"<ds:Signature xmlns:ds="http://www.w3.org/2000/09/xmldsig#"><ds:SignedInfo><ds:CanonicalizationMethod Algorithm="http://www.w3.org/2001/10/xml-exc-c14n#"></ds:CanonicalizationMethod><ds:SignatureMethod Algorithm="http://www.w3.org/2001/04/xmldsig-more#rsa-sha256"></ds:SignatureMethod><ds:Reference URI="#{id}"><ds:Transforms><ds:Transform Algorithm="http://www.w3.org/2000/09/xmldsig#enveloped-signature"></ds:Transform><ds:Transform Algorithm="http://www.w3.org/2001/10/xml-exc-c14n#"></ds:Transform></ds:Transforms><ds:DigestMethod Algorithm="http://www.w3.org/2001/04/xmlenc#sha256"></ds:DigestMethod><ds:DigestValue>{digest}</ds:DigestValue></ds:Reference></ds:SignedInfo><ds:SignatureValue>{signature}</ds:SignatureValue></ds:Signature>"##
            )
        };
        let signed_info = signature_element_of(&digest, "").replacen(
            "<ds:SignedInfo>",
            r##"<ds:SignedInfo xmlns:ds="http://www.w3.org/2000/09/xmldsig#">"##,
            1,
        );
        let start = signed_info
            .find("<ds:SignedInfo")
            .expect("the signature holds a SignedInfo");
        let end = signed_info
            .find("</ds:SignedInfo>")
            .expect("the signature holds a SignedInfo")
            + "</ds:SignedInfo>".len();
        let signature =
            SigningKey::<Sha256>::new(key.clone()).sign(signed_info[start..end].as_bytes());

        signature_element_of(&digest, &STANDARD.encode(signature.to_bytes()))
    }

    /// Returns a logout request of the asserting party, signed with the key.
    ///
    /// The message is written in the canonical form itself: the root declares the
    /// prefix it uses, and the assertion namespace is declared on the elements
    /// that use it, so the digest the signature carries is the digest of the
    /// element the asserting party signed.
    fn logout_request(key: Option<&rsa::RsaPrivateKey>) -> String {
        let message = format!(
            r##"<saml2p:LogoutRequest xmlns:saml2p="urn:oasis:names:tc:SAML:2.0:protocol" Destination="{SP_LOGOUT_LOCATION}" ID="the-logout-request" IssueInstant="2026-10-19T13:00:00Z" Version="2.0"><saml2:Issuer xmlns:saml2="urn:oasis:names:tc:SAML:2.0:assertion">https://idp.example.org</saml2:Issuer><saml2:NameID xmlns:saml2="urn:oasis:names:tc:SAML:2.0:assertion">Max</saml2:NameID></saml2p:LogoutRequest>"##
        );
        let Some(key) = key else {
            return message;
        };
        let signature = signature_of(&message, "the-logout-request", key);

        message.replace(
            "</saml2p:LogoutRequest>",
            &format!("{signature}</saml2p:LogoutRequest>"),
        )
    }

    /// Returns a logout response of the asserting party, signed with the key.
    fn logout_response(key: &rsa::RsaPrivateKey) -> String {
        let message = format!(
            r##"<saml2p:LogoutResponse xmlns:saml2p="urn:oasis:names:tc:SAML:2.0:protocol" Destination="{SP_LOGOUT_LOCATION}" ID="the-logout-response" InResponseTo="our-logout" IssueInstant="2026-10-19T13:00:00Z" Version="2.0"><saml2:Issuer xmlns:saml2="urn:oasis:names:tc:SAML:2.0:assertion">https://idp.example.org</saml2:Issuer><saml2p:Status><saml2p:StatusCode Value="urn:oasis:names:tc:SAML:2.0:status:Success"></saml2p:StatusCode></saml2p:Status></saml2p:LogoutResponse>"##
        );
        let signature = signature_of(&message, "the-logout-response", key);

        message.replace(
            "</saml2p:LogoutResponse>",
            &format!("{signature}</saml2p:LogoutResponse>"),
        )
    }

    /// Returns an HTTP security whose SAML 2.0 logout is configured.
    fn http_security(key: &rsa::RsaPrivateKey) -> HttpSecurity {
        let mut http = HttpSecurity::new(AuthenticationManagerBuilder::default(), HashMap::new());
        http.saml2_logout(|logout| {
            logout
                .relying_party_registration_repository(Arc::new(
                    InMemoryRelyingPartyRegistrationRepository::new(vec![registration(key)]),
                ))
                .logout_success_url("/login?logout");
        });

        http
    }

    /// Returns a `POST` of a logout message to the endpoint of the application.
    fn request(parameter: &str, message: &str) -> Request {
        let value = STANDARD.encode(message.as_bytes());
        let body = format!("{}={}", parameter, urlencoding::encode(&value));
        let mut request = Request::builder()
            .method("POST")
            .uri(SP_LOGOUT_LOCATION)
            .header("host", "sp.example.org")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::empty())
            .expect("a valid request");
        request
            .extensions_mut()
            .insert(next_web_core::http::RequestBody::new(body.into_bytes()));

        request
    }

    /// Runs one request through the chain, and returns the answer.
    async fn run(mut http: HttpSecurity, mut request: Request) -> Response {
        let chain = http.build();
        let filters = Arc::new(chain.get_filters().to_vec());
        let mut response = Response::default();
        Chain::new(filters)
            .run(&mut request, &mut response)
            .await
            .expect("the chain runs");

        response
    }

    /// Returns the `Location` header of a response.
    fn redirect_of(response: &Response) -> Option<String> {
        response
            .headers()
            .get("location")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned)
    }

    /// Signs a user in, so a logout has a session to end.
    fn sign_in() {
        use crate::web::authentication::preauth::PreAuthenticatedAuthenticationToken;

        let authentication =
            PreAuthenticatedAuthenticationToken::new(Arc::new("Max".to_string()), None);
        SecurityContextHolder::get_context().set_authentication(Some(Arc::new(authentication)));
    }

    /// Returns whether a user is signed in.
    fn signed_in() -> bool {
        SecurityContextHolder::get_context()
            .get_authentication()
            .is_some()
    }

    #[test]
    fn the_configuration_installs_the_logout_filters() {
        let key = signing_key();
        let mut http = http_security(&key);
        let names = http
            .build()
            .get_filters()
            .iter()
            .map(|filter| filter.name().to_owned())
            .collect::<Vec<_>>();

        assert!(
            names.iter().any(|name| name == "Saml2LogoutRequestFilter"),
            "the filters were: {names:?}"
        );
        assert!(
            names.iter().any(|name| name == "Saml2LogoutResponseFilter"),
            "the filters were: {names:?}"
        );
    }

    #[tokio::test]
    async fn a_signed_logout_request_ends_the_session_and_is_answered() {
        sign_in();
        let key = signing_key();
        let response = run(
            http_security(&key),
            request("SAMLRequest", &logout_request(Some(&key))),
        )
        .await;
        let status = response.status().as_u16();
        let location = redirect_of(&response);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("the body is collected");
        let body = String::from_utf8_lossy(&body).to_string();
        let location = location.unwrap_or_else(|| {
            panic!("the request is answered with a redirect, the answer was {status}: {body}")
        });

        assert_eq!(status, 302, "it was: {location}");
        assert!(
            location.starts_with(IDP_LOGOUT_LOCATION),
            "it was: {location}"
        );
        assert!(
            location.contains("SAMLResponse="),
            "the asserting party is answered with a logout response, it was: {location}"
        );
        assert!(
            !signed_in(),
            "the session of the user ended with the logout request"
        );
    }

    #[tokio::test]
    async fn a_logout_request_without_a_signature_is_refused() {
        sign_in();
        let key = signing_key();
        let response = run(
            http_security(&key),
            request("SAMLRequest", &logout_request(None)),
        )
        .await;
        let status = response.status().as_u16();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("the body is collected");
        let body = String::from_utf8_lossy(&body).to_string();

        assert_eq!(
            status, 400,
            "an unsigned logout request is refused, the answer was: {body}"
        );
        assert!(
            signed_in(),
            "a logout request that is not believed does not end the session"
        );
    }

    #[tokio::test]
    async fn a_signed_logout_response_completes_the_logout() {
        sign_in();
        let key = signing_key();
        let response = run(
            http_security(&key),
            request("SAMLResponse", &logout_response(&key)),
        )
        .await;

        assert_eq!(
            response.status().as_u16(),
            302,
            "the response is answered with a redirect"
        );
        assert_eq!(
            redirect_of(&response),
            Some("/login?logout".to_owned()),
            "a completed logout sends the user to the success URL"
        );
        assert!(
            !signed_in(),
            "the session of the user ended with the logout response"
        );
    }
}
