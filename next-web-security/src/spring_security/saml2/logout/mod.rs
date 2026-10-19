//! The logout profile of the service provider.
//!
//! A single logout is two messages: the asserting party asks the application to
//! end the session of a user (`LogoutRequest`), and it answers the logout the
//! application started (`LogoutResponse`). Both are carried in a parameter —
//! `SAMLRequest` and `SAMLResponse` — and both may be signed by the asserting
//! party, which is what the filters of the profile check before anything else.

use std::io::Read;

use base64::{Engine, engine::general_purpose::STANDARD};
use quick_xml::{
    Reader,
    events::Event,
};

use crate::saml2::{
    error::{Saml2Error, Saml2ErrorKind},
    registration::Saml2MessageBinding,
};

/// The parameter a logout request is carried in.
pub const LOGOUT_REQUEST_PARAMETER: &str = "SAMLRequest";
/// The parameter a logout response is carried in.
pub const LOGOUT_RESPONSE_PARAMETER: &str = "SAMLResponse";

/// The status a logout response has to report to be accepted.
const STATUS_SUCCESS: &str = "urn:oasis:names:tc:SAML:2.0:status:Success";

/// Returns the XML of a message that was carried in a parameter.
///
/// The HTTP-POST binding carries the message as base64; the HTTP-Redirect binding
/// carries it as base64 of a deflated document.
///
/// # Errors
///
/// Returns [`Saml2ErrorKind::InvalidResponse`] when the parameter does not hold a
/// message of the binding.
pub fn decode_message(value: &str, binding: Saml2MessageBinding) -> Result<String, Saml2Error> {
    let decoded = STANDARD
        .decode(value.trim().as_bytes())
        .map_err(|error| invalid(format!("the message is not base64: {error}")))?;
    let document = match binding {
        Saml2MessageBinding::Post => decoded,
        Saml2MessageBinding::Redirect => {
            let mut document = Vec::new();
            flate2::read::DeflateDecoder::new(decoded.as_slice())
                .read_to_end(&mut document)
                .map_err(|error| invalid(format!("the message is not deflated: {error}")))?;

            document
        }
    };

    String::from_utf8(document)
        .map_err(|error| invalid(format!("the message is not UTF-8: {error}")))
}

/// Returns the error of a message that cannot be read.
fn invalid(reason: impl Into<String>) -> Saml2Error {
    Saml2Error::new(Saml2ErrorKind::InvalidResponse, reason.into())
}

/// Returns the error of a message whose signature does not hold.
fn not_believed(reason: impl Into<String>) -> Saml2Error {
    Saml2Error::new(Saml2ErrorKind::VerificationFailed, reason.into())
}

/// A logout request of the asserting party.
#[derive(Clone, Debug, Default)]
pub struct Saml2LogoutRequest {
    document: String,
    id: Option<String>,
    issuer: Option<String>,
    destination: Option<String>,
    name_id: Option<String>,
    not_on_or_after: Option<String>,
    signed: bool,
}

impl Saml2LogoutRequest {
    /// Parses a logout request from the XML of the message.
    ///
    /// # Errors
    ///
    /// Returns [`Saml2ErrorKind::InvalidResponse`] when the document is not a
    /// logout request.
    pub fn parse(xml: &str) -> Result<Self, Saml2Error> {
        let mut request = Self {
            document: xml.to_string(),
            ..Self::default()
        };
        let mut root_seen = false;
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);
        let mut text = String::new();
        let mut target: Option<&str> = None;

        loop {
            match reader.read_event() {
                Ok(Event::Start(element)) | Ok(Event::Empty(element)) => {
                    match local_name(element.name().as_ref()).as_str() {
                        "LogoutRequest" => {
                            root_seen = true;
                            request.id = attribute(&element, "ID")?;
                            request.destination = attribute(&element, "Destination")?;
                            request.not_on_or_after = attribute(&element, "NotOnOrAfter")?;
                        }
                        "Issuer" => target = Some("issuer"),
                        "NameID" => target = Some("name_id"),
                        "Signature" => request.signed = true,
                        _ => {}
                    }
                }
                Ok(Event::Text(span)) => {
                    if target.is_some() {
                        text.push_str(&span.unescape().unwrap_or_default());
                    }
                }
                Ok(Event::End(element)) => {
                    let name = local_name(element.name().as_ref());
                    let value = std::mem::take(&mut text);
                    match (name.as_str(), target.take()) {
                        ("Issuer", Some(_)) => request.issuer = Some(value),
                        ("NameID", Some(_)) => request.name_id = Some(value),
                        _ => {}
                    }
                }
                Ok(Event::Eof) => break,
                Ok(_) => {}
                Err(error) => {
                    return Err(invalid(format!(
                        "the logout request is not readable XML: {error}"
                    )));
                }
            }
        }

        if !root_seen {
            return Err(invalid("the document is not a logout request"));
        }

        Ok(request)
    }

    /// Parses a logout request that was carried in a parameter.
    pub fn from_parameter(value: &str, binding: Saml2MessageBinding) -> Result<Self, Saml2Error> {
        Self::parse(&decode_message(value, binding)?)
    }

    /// Returns the document the request was read from.
    pub fn document(&self) -> &str {
        &self.document
    }

    /// Returns the identifier of the request.
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    /// Returns the asserting party of the request.
    pub fn issuer(&self) -> Option<&str> {
        self.issuer.as_deref()
    }

    /// Returns the endpoint the request is meant for.
    pub fn destination(&self) -> Option<&str> {
        self.destination.as_deref()
    }

    /// Returns the user whose session ends.
    pub fn name_id(&self) -> Option<&str> {
        self.name_id.as_deref()
    }

    /// Returns the instant after which the request is not accepted.
    pub fn not_on_or_after(&self) -> Option<&str> {
        self.not_on_or_after.as_deref()
    }

    /// Returns whether the request carries a signature.
    pub fn is_signed(&self) -> bool {
        self.signed
    }
}

/// A logout response of the asserting party.
#[derive(Clone, Debug, Default)]
pub struct Saml2LogoutResponse {
    document: String,
    id: Option<String>,
    issuer: Option<String>,
    destination: Option<String>,
    in_response_to: Option<String>,
    status_code: Option<String>,
    signed: bool,
}

impl Saml2LogoutResponse {
    /// Parses a logout response from the XML of the message.
    ///
    /// # Errors
    ///
    /// Returns [`Saml2ErrorKind::InvalidResponse`] when the document is not a
    /// logout response.
    pub fn parse(xml: &str) -> Result<Self, Saml2Error> {
        let mut response = Self {
            document: xml.to_string(),
            ..Self::default()
        };
        let mut root_seen = false;
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);
        let mut text = String::new();
        let mut target: Option<&str> = None;

        loop {
            match reader.read_event() {
                Ok(Event::Start(element)) | Ok(Event::Empty(element)) => {
                    match local_name(element.name().as_ref()).as_str() {
                        "LogoutResponse" => {
                            root_seen = true;
                            response.id = attribute(&element, "ID")?;
                            response.destination = attribute(&element, "Destination")?;
                            response.in_response_to = attribute(&element, "InResponseTo")?;
                        }
                        "StatusCode" => response.status_code = attribute(&element, "Value")?,
                        "Issuer" => target = Some("issuer"),
                        "Signature" => response.signed = true,
                        _ => {}
                    }
                }
                Ok(Event::Text(span)) => {
                    if target.is_some() {
                        text.push_str(&span.unescape().unwrap_or_default());
                    }
                }
                Ok(Event::End(element)) => {
                    let name = local_name(element.name().as_ref());
                    let value = std::mem::take(&mut text);
                    if let ("Issuer", Some(_)) = (name.as_str(), target.take()) {
                        response.issuer = Some(value);
                    }
                }
                Ok(Event::Eof) => break,
                Ok(_) => {}
                Err(error) => {
                    return Err(invalid(format!(
                        "the logout response is not readable XML: {error}"
                    )));
                }
            }
        }

        if !root_seen {
            return Err(invalid("the document is not a logout response"));
        }

        Ok(response)
    }

    /// Parses a logout response that was carried in a parameter.
    pub fn from_parameter(value: &str, binding: Saml2MessageBinding) -> Result<Self, Saml2Error> {
        Self::parse(&decode_message(value, binding)?)
    }

    /// Returns the document the response was read from.
    pub fn document(&self) -> &str {
        &self.document
    }

    /// Returns the identifier of the response.
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    /// Returns the asserting party of the response.
    pub fn issuer(&self) -> Option<&str> {
        self.issuer.as_deref()
    }

    /// Returns the endpoint the response is meant for.
    pub fn destination(&self) -> Option<&str> {
        self.destination.as_deref()
    }

    /// Returns the request the response answers.
    pub fn in_response_to(&self) -> Option<&str> {
        self.in_response_to.as_deref()
    }

    /// Returns the status of the response.
    pub fn status_code(&self) -> Option<&str> {
        self.status_code.as_deref()
    }

    /// Returns whether the response reports success.
    pub fn is_success(&self) -> bool {
        self.status_code.as_deref() == Some(STATUS_SUCCESS)
    }

    /// Returns whether the response carries a signature.
    pub fn is_signed(&self) -> bool {
        self.signed
    }
}

/// Returns the local name of a qualified name.
fn local_name(name: &[u8]) -> String {
    let qualified = String::from_utf8_lossy(name);
    qualified
        .rsplit(':')
        .next()
        .unwrap_or(&qualified)
        .to_string()
}

/// Returns the value of an attribute of an element.
fn attribute(element: &quick_xml::events::BytesStart, name: &str) -> Result<Option<String>, Saml2Error> {
    for attribute in element.attributes().flatten() {
        let key = String::from_utf8_lossy(attribute.key.as_ref()).to_string();
        if key == name {
            return Ok(Some(attribute.unescape_value().unwrap_or_default().to_string()));
        }
    }

    Ok(None)
}

/// Returns the error of a message the profile refuses.
pub(crate) fn refused(reason: impl Into<String>) -> Saml2Error {
    not_believed(reason)
}

/// Returns an identifier for a logout message the application sends.
pub(crate) fn new_id() -> String {
    let bytes: [u8; 16] = rand::random();
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_logout_request_is_read() {
        let request = Saml2LogoutRequest::parse(
            r#"<saml2p:LogoutRequest xmlns:saml2p="urn:oasis:names:tc:SAML:2.0:protocol" ID="the-request" Destination="https://sp.example.org/logout/saml2/slo" NotOnOrAfter="2099-01-01T00:00:00Z"><saml2:Issuer xmlns:saml2="urn:oasis:names:tc:SAML:2.0:assertion">https://idp.example.org</saml2:Issuer><saml2:NameID xmlns:saml2="urn:oasis:names:tc:SAML:2.0:assertion">Max</saml2:NameID></saml2p:LogoutRequest>"#,
        )
        .expect("the request parses");

        assert_eq!(request.id(), Some("the-request"));
        assert_eq!(request.issuer(), Some("https://idp.example.org"));
        assert_eq!(request.name_id(), Some("Max"));
        assert_eq!(
            request.destination(),
            Some("https://sp.example.org/logout/saml2/slo")
        );
    }

    #[test]
    fn a_logout_response_is_read() {
        let response = Saml2LogoutResponse::parse(
            r#"<saml2p:LogoutResponse xmlns:saml2p="urn:oasis:names:tc:SAML:2.0:protocol" xmlns:saml2="urn:oasis:names:tc:SAML:2.0:assertion" ID="the-response" InResponseTo="our-request"><saml2:Issuer>https://idp.example.org</saml2:Issuer><saml2p:Status><saml2p:StatusCode Value="urn:oasis:names:tc:SAML:2.0:status:Success"></saml2p:StatusCode></saml2p:Status></saml2p:LogoutResponse>"#,
        )
        .expect("the response parses");

        assert_eq!(response.in_response_to(), Some("our-request"));
        assert!(response.is_success(), "the response reports success");
    }

    #[test]
    fn a_document_that_is_not_a_logout_message_is_refused() {
        let error = Saml2LogoutRequest::parse("<Something />").expect_err("the document is not one");

        assert_eq!(error.kind(), Saml2ErrorKind::InvalidResponse);
    }

    #[test]
    fn a_posted_message_is_read_from_base64() {
        let document = r#"<saml2p:LogoutRequest xmlns:saml2p="urn:oasis:names:tc:SAML:2.0:protocol" ID="the-request"></saml2p:LogoutRequest>"#;
        let value = STANDARD.encode(document.as_bytes());

        let request = Saml2LogoutRequest::from_parameter(&value, Saml2MessageBinding::Post)
            .expect("the message is read");

        assert_eq!(request.id(), Some("the-request"));
    }

    #[test]
    fn a_redirected_message_is_read_from_deflated_base64() {
        use std::io::Write;

        let document = r#"<saml2p:LogoutRequest xmlns:saml2p="urn:oasis:names:tc:SAML:2.0:protocol" ID="the-request"></saml2p:LogoutRequest>"#;
        let mut deflated = Vec::new();
        let mut encoder =
            flate2::write::DeflateEncoder::new(&mut deflated, flate2::Compression::default());
        encoder
            .write_all(document.as_bytes())
            .expect("the document is deflated");
        encoder.finish().expect("the document is deflated");
        let value = STANDARD.encode(&deflated);

        let request = Saml2LogoutRequest::from_parameter(&value, Saml2MessageBinding::Redirect)
            .expect("the message is read");

        assert_eq!(request.id(), Some("the-request"));
    }
}
