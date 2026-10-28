use std::io::Write;

use base64::Engine;
use chrono::Utc;
use flate2::{write::DeflateEncoder, Compression};

use crate::saml2::{
    error::{Saml2Error, Saml2ErrorKind},
    registration::RelyingPartyRegistration,
    xml::{escape_xml, resolve_placeholders},
};

/// The namespace an authentication request is written in.
const SAML2_PROTOCOL_NAMESPACE: &str = "urn:oasis:names:tc:SAML:2.0:protocol";

/// The namespace the issuer of an authentication request is written in.
const SAML2_ASSERTION_NAMESPACE: &str = "urn:oasis:names:tc:SAML:2.0:assertion";

/// An authentication request the service provider sends to start a login.
///
/// The request names the service provider, the endpoint the assertion is posted
/// back to, and the asserting party it is addressed to. It carries no user
/// information: the asserting party decides who the user is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Saml2AuthenticationRequest {
    id: String,
    issue_instant: String,
    destination: String,
    issuer: String,
    assertion_consumer_service_url: String,
    protocol_binding: String,
}

impl Saml2AuthenticationRequest {
    /// Creates a request for the given registration.
    ///
    /// `base_url` replaces the `{baseUrl}` placeholder of the registration, so
    /// the endpoints the request names are the ones the client reached the
    /// application on.
    pub fn create(
        registration: &RelyingPartyRegistration,
        base_url: &str,
    ) -> Result<Self, Saml2Error> {
        let Some(destination) = registration.single_sign_on_service_location() else {
            return Err(Saml2Error::new(
                Saml2ErrorKind::MissingConfiguration,
                "the registration has no single sign-on service location",
            ));
        };

        Ok(Self {
            id: format!("_{:016x}{:016x}", rand::random::<u64>(), rand::random::<u64>()),
            issue_instant: Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            destination: resolve_placeholders(destination, registration, base_url),
            issuer: resolve_placeholders(registration.entity_id(), registration, base_url),
            assertion_consumer_service_url: resolve_placeholders(
                registration.assertion_consumer_service_location(),
                registration,
                base_url,
            ),
            protocol_binding: registration
                .assertion_consumer_service_binding()
                .urn()
                .to_string(),
        })
    }

    /// Returns the identifier the response is expected to answer.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the request as the XML that travels to the asserting party.
    pub fn to_xml(&self) -> String {
        format!(
            "<saml2p:AuthnRequest xmlns:saml2p=\"{SAML2_PROTOCOL_NAMESPACE}\" \
             xmlns:saml2=\"{SAML2_ASSERTION_NAMESPACE}\" ID=\"{}\" Version=\"2.0\" \
             IssueInstant=\"{}\" Destination=\"{}\" ProtocolBinding=\"{}\" \
             AssertionConsumerServiceURL=\"{}\"><saml2:Issuer>{}</saml2:Issuer></saml2p:AuthnRequest>",
            escape_xml(&self.id),
            escape_xml(&self.issue_instant),
            escape_xml(&self.destination),
            escape_xml(&self.protocol_binding),
            escape_xml(&self.assertion_consumer_service_url),
            escape_xml(&self.issuer),
        )
    }

    /// Returns the URL that starts the login at the asserting party.
    ///
    /// The request travels over the HTTP-Redirect binding: deflated, base64
    /// encoded, and put in the query, with the state to return to beside it.
    pub fn to_redirect_url(&self, relay_state: Option<&str>) -> Result<String, Saml2Error> {
        let encoded = base64::engine::general_purpose::STANDARD.encode(deflate(&self.to_xml())?);

        let mut url = format!(
            "{}?SAMLRequest={}",
            self.destination,
            urlencoding::encode(&encoded)
        );
        if let Some(relay_state) = relay_state {
            url.push_str("&RelayState=");
            url.push_str(&urlencoding::encode(relay_state));
        }

        Ok(url)
    }
}

/// Deflates a message the way the HTTP-Redirect binding requires it.
fn deflate(xml: &str) -> Result<Vec<u8>, Saml2Error> {
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(xml.as_bytes()).map_err(|error| {
        Saml2Error::new(
            Saml2ErrorKind::Encoding,
            format!("the authentication request cannot be deflated: {error}"),
        )
    })?;

    encoder.finish().map_err(|error| {
        Saml2Error::new(
            Saml2ErrorKind::Encoding,
            format!("the authentication request cannot be deflated: {error}"),
        )
    })
}

#[cfg(test)]
mod tests {
    use base64::Engine;
    use flate2::read::DeflateDecoder;
    use std::io::Read;

    use super::*;
    use crate::saml2::registration::RelyingPartyRegistration;

    /// Returns a registration of a service provider configured for an identity
    /// provider.
    fn registration() -> RelyingPartyRegistration {
        RelyingPartyRegistration::with_registration_id("okta")
            .entity_id("sp-entity-id")
            .assertion_consumer_service_location("https://sp.example.org/login/saml2/sso/okta")
            .asserting_party_entity_id("idp-entity-id")
            .single_sign_on_service_location("https://idp.example.org/sso")
            .build()
    }

    /// Returns the value of a query parameter of a URL.
    fn query_value(url: &str, name: &str) -> Option<String> {
        let query = url.split_once('?')?.1;

        query.split('&').find_map(|pair| {
            let (key, value) = pair.split_once('=')?;

            (key == name).then(|| urlencoding::decode(value).ok().map(|value| value.into_owned()))?
        })
    }

    #[test]
    fn the_request_carries_the_service_provider() {
        let request =
            Saml2AuthenticationRequest::create(&registration(), "https://sp.example.org")
                .expect("the registration is complete");

        let xml = request.to_xml();

        assert!(xml.contains(r#"ID="_"#), "xml was: {xml}");
        assert!(
            xml.contains(r#"Destination="https://idp.example.org/sso""#),
            "xml was: {xml}"
        );
        assert!(
            xml.contains(
                r#"AssertionConsumerServiceURL="https://sp.example.org/login/saml2/sso/okta""#
            ),
            "xml was: {xml}"
        );
        assert!(
            xml.contains(r#"ProtocolBinding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST""#),
            "xml was: {xml}"
        );
        assert!(
            xml.contains("<saml2:Issuer>sp-entity-id</saml2:Issuer>"),
            "xml was: {xml}"
        );
    }

    #[test]
    fn the_redirect_url_carries_the_deflated_request() {
        let request =
            Saml2AuthenticationRequest::create(&registration(), "https://sp.example.org")
                .expect("the registration is complete");

        let url = request
            .to_redirect_url(None)
            .expect("the request encodes");
        let encoded = query_value(&url, "SAMLRequest").expect("the request is in the query");
        let deflated = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .expect("the request is base64");
        let mut xml = String::new();
        DeflateDecoder::new(deflated.as_slice())
            .read_to_string(&mut xml)
            .expect("the request is deflated");

        assert!(url.starts_with("https://idp.example.org/sso?SAMLRequest="), "url was: {url}");
        assert_eq!(xml, request.to_xml());
    }

    #[test]
    fn the_redirect_url_carries_the_relay_state() {
        let request =
            Saml2AuthenticationRequest::create(&registration(), "https://sp.example.org")
                .expect("the registration is complete");

        let url = request
            .to_redirect_url(Some("/protected page"))
            .expect("the request encodes");

        assert_eq!(
            query_value(&url, "RelayState").as_deref(),
            Some("/protected page")
        );
    }

    #[test]
    fn a_registration_without_a_single_sign_on_location_is_rejected() {
        let registration = RelyingPartyRegistration::with_registration_id("okta")
            .entity_id("sp-entity-id")
            .build();

        let error = Saml2AuthenticationRequest::create(&registration, "https://sp.example.org")
            .expect_err("the identity provider is not configured");

        assert_eq!(error.kind(), Saml2ErrorKind::MissingConfiguration);
    }
}
