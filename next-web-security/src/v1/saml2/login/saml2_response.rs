use quick_xml::{
    events::{BytesStart, Event},
    Reader,
};

use crate::saml2::error::{Saml2Error, Saml2ErrorKind};

/// A SAML 2.0 response, as much of it as the service provider reads.
///
/// The response is parsed, not trusted: what makes it acceptable — a signature
/// over what it claims, conditions that hold now, an audience that is this
/// service provider — is decided by the verifier the application supplies.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Saml2Response {
    /// The document the response was read from, because the signature of the
    /// response covers the document, not the fields read out of it.
    document: String,
    destination: Option<String>,
    in_response_to: Option<String>,
    issuer: Option<String>,
    status_code: Option<String>,
    assertion_id: Option<String>,
    name_id: Option<String>,
    audience: Option<String>,
    recipient: Option<String>,
    not_before: Option<String>,
    not_on_or_after: Option<String>,
    attributes: Vec<(String, Vec<String>)>,
    signed: bool,
}

impl Saml2Response {
    /// Returns the document the response was read from.
    pub fn document(&self) -> &str {
        &self.document
    }
}

impl Saml2Response {
    /// Parses a response document.
    ///
    /// # Errors
    ///
    /// Returns [`Saml2ErrorKind::InvalidResponse`] when the document is not XML,
    /// or its root is not a SAML 2.0 response.
    pub fn parse(xml: &str) -> Result<Self, Saml2Error> {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut response = Self {
            document: xml.to_string(),
            ..Self::default()
        };
        let mut root_seen = false;
        let mut text = String::new();
        let mut target: Option<&str> = None;
        let mut attribute_name: Option<String> = None;
        let mut attribute_values: Vec<String> = Vec::new();

        loop {
            match reader.read_event() {
                Ok(Event::Start(element)) | Ok(Event::Empty(element)) => {
                    match local_name(element.name().as_ref()).as_str() {
                        "Response" => {
                            root_seen = true;
                            response.destination = attribute(&element, "Destination")?;
                            response.in_response_to = attribute(&element, "InResponseTo")?;
                        }
                        "Issuer" if response.issuer.is_none() => target = Some("issuer"),
                        "StatusCode" => response.status_code = attribute(&element, "Value")?,
                        "Conditions" => {
                            response.not_before = attribute(&element, "NotBefore")?;
                            response.not_on_or_after = attribute(&element, "NotOnOrAfter")?;
                        }
                        "SubjectConfirmationData" => {
                            response.recipient = attribute(&element, "Recipient")?;
                            if response.not_on_or_after.is_none() {
                                response.not_on_or_after = attribute(&element, "NotOnOrAfter")?;
                            }
                        }
                        "Audience" => target = Some("audience"),
                        "NameID" => target = Some("name_id"),
                        "Assertion" => response.assertion_id = attribute(&element, "ID")?,
                        "Attribute" => {
                            attribute_name = attribute(&element, "Name")?;
                            attribute_values.clear();
                        }
                        "AttributeValue" => target = Some("attribute_value"),
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

                    match (name.as_str(), target.take()) {
                        ("Issuer", Some(_)) => response.issuer = Some(value),
                        ("NameID", Some(_)) => response.name_id = Some(value),
                        ("Audience", Some(_)) => response.audience = Some(value),
                        ("AttributeValue", Some(_)) => attribute_values.push(value),
                        ("Attribute", _) => {
                            if let Some(name) = attribute_name.take() {
                                response
                                    .attributes
                                    .push((name, std::mem::take(&mut attribute_values)));
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::Eof) => break,
                Ok(_) => {}
                Err(error) => {
                    return Err(Saml2Error::new(
                        Saml2ErrorKind::InvalidResponse,
                        format!("the response is not readable XML: {error}"),
                    ));
                }
            }
        }

        if !root_seen {
            return Err(Saml2Error::new(
                Saml2ErrorKind::InvalidResponse,
                "the document is not a SAML 2.0 response",
            ));
        }

        Ok(response)
    }

    /// Returns the endpoint the response was addressed to.
    pub fn destination(&self) -> Option<&str> {
        self.destination.as_deref()
    }

    /// Returns the identifier of the request the response answers.
    pub fn in_response_to(&self) -> Option<&str> {
        self.in_response_to.as_deref()
    }

    /// Returns the entity id of the asserting party.
    pub fn issuer(&self) -> Option<&str> {
        self.issuer.as_deref()
    }

    /// Returns the status the asserting party reports.
    pub fn status_code(&self) -> Option<&str> {
        self.status_code.as_deref()
    }

    /// Returns the name the subject of the assertion is known by.
    pub fn name_id(&self) -> Option<&str> {
        self.name_id.as_deref()
    }

    /// Returns the identifier of the assertion, which is what a replay is
    /// recognised by.
    pub fn assertion_id(&self) -> Option<&str> {
        self.assertion_id.as_deref()
    }

    /// Returns the audience the assertion is meant for.
    pub fn audience(&self) -> Option<&str> {
        self.audience.as_deref()
    }

    /// Returns the endpoint the assertion may be delivered to.
    pub fn recipient(&self) -> Option<&str> {
        self.recipient.as_deref()
    }

    /// Returns the instant the assertion becomes valid.
    pub fn not_before(&self) -> Option<&str> {
        self.not_before.as_deref()
    }

    /// Returns the instant the assertion stops being valid.
    pub fn not_on_or_after(&self) -> Option<&str> {
        self.not_on_or_after.as_deref()
    }

    /// Returns the values of the named attribute.
    pub fn attribute_values(&self, name: &str) -> Vec<String> {
        self.attributes
            .iter()
            .filter(|(attribute, _)| attribute == name)
            .flat_map(|(_, values)| values.iter().cloned())
            .collect()
    }

    /// Returns whether the response carries a signature.
    ///
    /// A response without one can never be accepted: a verifier needs something
    /// to verify.
    pub fn is_signed(&self) -> bool {
        self.signed
    }
}

/// Returns the name of an element without its namespace prefix.
fn local_name(name: &[u8]) -> String {
    let name = String::from_utf8_lossy(name);

    name.rsplit_once(':')
        .map(|(_, local)| local.to_owned())
        .unwrap_or_else(|| name.into_owned())
}

/// Returns an attribute of an element by its local name.
fn attribute(element: &BytesStart, name: &str) -> Result<Option<String>, Saml2Error> {
    for attribute in element.attributes() {
        let attribute = attribute.map_err(|error| {
            Saml2Error::new(
                Saml2ErrorKind::InvalidResponse,
                format!("the response carries an unreadable attribute: {error}"),
            )
        })?;

        if local_name(attribute.key.as_ref()) == name {
            return Ok(Some(String::from_utf8_lossy(&attribute.value).into_owned()));
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The response an asserting party posts back for one registration.
    const RESPONSE: &str = r#"<saml2p:Response xmlns:saml2p="urn:oasis:names:tc:SAML:2.0:protocol" xmlns:saml2="urn:oasis:names:tc:SAML:2.0:assertion" Destination="https://sp.example.org/login/saml2/sso/okta" InResponseTo="_abc"><saml2:Issuer>idp-entity-id</saml2:Issuer><saml2p:Status><saml2p:StatusCode Value="urn:oasis:names:tc:SAML:2.0:status:Success"/></saml2p:Status><saml2:Assertion><saml2:Issuer>idp-entity-id</saml2:Issuer><saml2:Subject><saml2:NameID Format="urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress">max@example.org</saml2:NameID><saml2:SubjectConfirmation><saml2:SubjectConfirmationData NotOnOrAfter="2026-10-18T12:10:00Z" Recipient="https://sp.example.org/login/saml2/sso/okta"/></saml2:SubjectConfirmation></saml2:Subject><saml2:Conditions NotBefore="2026-10-18T12:00:00Z" NotOnOrAfter="2026-10-18T12:05:00Z"><saml2:AudienceRestriction><saml2:Audience>sp-entity-id</saml2:Audience></saml2:AudienceRestriction></saml2:Conditions><saml2:AttributeStatement><saml2:Attribute Name="email"><saml2:AttributeValue>max@example.org</saml2:AttributeValue></saml2:Attribute></saml2:AttributeStatement></saml2:Assertion></saml2p:Response>"#;

    #[test]
    fn the_response_carries_the_message_of_the_asserting_party() {
        let response = Saml2Response::parse(RESPONSE).expect("the response parses");

        assert_eq!(
            response.destination(),
            Some("https://sp.example.org/login/saml2/sso/okta")
        );
        assert_eq!(response.in_response_to(), Some("_abc"));
        assert_eq!(response.issuer(), Some("idp-entity-id"));
        assert_eq!(
            response.status_code(),
            Some("urn:oasis:names:tc:SAML:2.0:status:Success")
        );
    }

    #[test]
    fn the_assertion_carries_the_subject_and_its_conditions() {
        let response = Saml2Response::parse(RESPONSE).expect("the response parses");

        assert_eq!(response.name_id(), Some("max@example.org"));
        assert_eq!(response.audience(), Some("sp-entity-id"));
        assert_eq!(
            response.recipient(),
            Some("https://sp.example.org/login/saml2/sso/okta")
        );
        assert_eq!(response.not_before(), Some("2026-10-18T12:00:00Z"));
        assert_eq!(response.not_on_or_after(), Some("2026-10-18T12:05:00Z"));
    }

    #[test]
    fn the_assertion_carries_the_attribute_values() {
        let response = Saml2Response::parse(RESPONSE).expect("the response parses");

        assert_eq!(
            response.attribute_values("email"),
            vec!["max@example.org".to_owned()]
        );
    }

    #[test]
    fn a_response_without_a_signature_says_so() {
        let response = Saml2Response::parse(RESPONSE).expect("the response parses");

        assert!(!response.is_signed());
    }

    #[test]
    fn a_response_with_a_signature_says_so() {
        let signed = RESPONSE.replace(
            "<saml2:Issuer>idp-entity-id</saml2:Issuer>",
            "<saml2:Issuer>idp-entity-id</saml2:Issuer><ds:Signature xmlns:ds=\"http://www.w3.org/2000/09/xmldsig#\"/>",
        );

        let response = Saml2Response::parse(&signed).expect("the response parses");

        assert!(response.is_signed());
    }

    #[test]
    fn a_document_that_is_not_a_response_is_rejected() {
        let error = Saml2Response::parse("<not-saml/>").expect_err("the document is not a response");

        assert_eq!(error.kind(), crate::saml2::Saml2ErrorKind::InvalidResponse);
    }
}
