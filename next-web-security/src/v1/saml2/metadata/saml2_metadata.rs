use crate::saml2::registration::RelyingPartyRegistration;

/// The namespace a metadata document is written in.
pub const SAML2_METADATA_NAMESPACE: &str = "urn:oasis:names:tc:SAML:2.0:metadata";

/// The namespace the key material is written in.
pub const XML_SIGNATURE_NAMESPACE: &str = "http://www.w3.org/2000/09/xmldsig#";

/// The protocol the service provider speaks.
pub const SAML2_PROTOCOL_NAMESPACE: &str = "urn:oasis:names:tc:SAML:2.0:protocol";

/// The declaration every document starts with.
const XML_DECLARATION: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n";

/// Renders the metadata of relying party registrations.
///
/// A document describes one relying party as an `EntityDescriptor`, and several
/// relying parties as an `EntitiesDescriptor` that holds one `EntityDescriptor`
/// each. The document is not signed: signing a metadata document needs a key
/// the service provider is given, and nothing in the current slices asks for
/// one.
#[derive(Clone, Debug, Default)]
pub struct Saml2Metadata;

impl Saml2Metadata {
    /// Renders the metadata of one registration.
    ///
    /// `base_url` is the base URL of the request the document is rendered for;
    /// it replaces the `{baseUrl}` placeholder of the registration, and the
    /// registration id replaces `{registrationId}`.
    pub fn resolve(&self, registration: &RelyingPartyRegistration, base_url: &str) -> String {
        let mut document = String::from(XML_DECLARATION);
        document.push_str(&self.entity_descriptor(registration, base_url, 0, true));

        document
    }

    /// Renders the metadata of every registration.
    ///
    /// A single registration is rendered as a bare `EntityDescriptor`, the way
    /// the reference implementation answers that request, and only several
    /// registrations are wrapped in an `EntitiesDescriptor`.
    pub fn resolve_all(
        &self,
        registrations: &[RelyingPartyRegistration],
        base_url: &str,
    ) -> String {
        if registrations.len() == 1 {
            return self.resolve(&registrations[0], base_url);
        }

        let mut document = String::from(XML_DECLARATION);
        document.push_str(&format!(
            "<md:EntitiesDescriptor xmlns:md=\"{SAML2_METADATA_NAMESPACE}\">\n"
        ));
        for registration in registrations {
            document.push_str(&self.entity_descriptor(registration, base_url, 1, false));
        }
        document.push_str("</md:EntitiesDescriptor>\n");

        document
    }

    /// Renders one `EntityDescriptor`, at the given indentation level.
    ///
    /// The namespace is declared on the descriptor that is the root of the
    /// document, and inherited by the descriptors an `EntitiesDescriptor` holds.
    fn entity_descriptor(
        &self,
        registration: &RelyingPartyRegistration,
        base_url: &str,
        level: usize,
        declare_namespace: bool,
    ) -> String {
        let indent = "  ".repeat(level);
        let namespace = if declare_namespace {
            format!(" xmlns:md=\"{SAML2_METADATA_NAMESPACE}\"")
        } else {
            String::new()
        };
        let entity_id = placeholder(registration.entity_id(), registration, base_url);

        let mut document = format!(
            "{indent}<md:EntityDescriptor{namespace} entityID=\"{}\">\n",
            escape_xml(&entity_id)
        );
        document.push_str(&self.sp_sso_descriptor(registration, base_url, level + 1));
        document.push_str(&format!("{indent}</md:EntityDescriptor>\n"));

        document
    }

    /// Renders the `SPSSODescriptor` of one registration.
    ///
    /// The children follow the order the metadata schema declares: the keys
    /// first, then the logout endpoint, the requested name id format, and the
    /// assertion consumer endpoint.
    fn sp_sso_descriptor(
        &self,
        registration: &RelyingPartyRegistration,
        base_url: &str,
        level: usize,
    ) -> String {
        let indent = "  ".repeat(level);
        let mut document = format!(
            "{indent}<md:SPSSODescriptor protocolSupportEnumeration=\"{SAML2_PROTOCOL_NAMESPACE}\">\n"
        );

        if let Some(certificate) = registration.signing_certificate() {
            document.push_str(&self.key_descriptor("signing", certificate, level + 1));
        }
        if let Some(certificate) = registration.encryption_certificate() {
            document.push_str(&self.key_descriptor("encryption", certificate, level + 1));
        }

        if let Some(location) = registration.single_logout_service_location() {
            let location = placeholder(location, registration, base_url);
            let response_location = registration
                .single_logout_service_response_location()
                .map(|value| placeholder(value, registration, base_url))
                .unwrap_or_else(|| location.clone());

            document.push_str(&format!(
                "{indent}  <md:SingleLogoutService Binding=\"{}\" Location=\"{}\" ResponseLocation=\"{}\"/>\n",
                registration.single_logout_service_binding().urn(),
                escape_xml(&location),
                escape_xml(&response_location)
            ));
        }

        if let Some(name_id_format) = registration.name_id_format() {
            document.push_str(&format!(
                "{indent}  <md:NameIDFormat>{}</md:NameIDFormat>\n",
                escape_xml(name_id_format)
            ));
        }

        let location = placeholder(
            registration.assertion_consumer_service_location(),
            registration,
            base_url,
        );
        document.push_str(&format!(
            "{indent}  <md:AssertionConsumerService Binding=\"{}\" Location=\"{}\" index=\"1\"/>\n",
            registration.assertion_consumer_service_binding().urn(),
            escape_xml(&location)
        ));

        document.push_str(&format!("{indent}</md:SPSSODescriptor>\n"));

        document
    }

    /// Renders one `KeyDescriptor` around the public part of a certificate.
    fn key_descriptor(&self, usage: &str, certificate: &str, level: usize) -> String {
        let indent = "  ".repeat(level);
        let body = certificate_body(certificate);

        format!(
            "{indent}<md:KeyDescriptor use=\"{usage}\">\n\
             {indent}  <ds:KeyInfo xmlns:ds=\"{XML_SIGNATURE_NAMESPACE}\">\n\
             {indent}    <ds:X509Data>\n\
             {indent}      <ds:X509Certificate>{body}</ds:X509Certificate>\n\
             {indent}    </ds:X509Data>\n\
             {indent}  </ds:KeyInfo>\n\
             {indent}</md:KeyDescriptor>\n"
        )
    }
}

/// Replaces the placeholders a registration field may carry.
///
/// A field is a template, not an absolute URL: the same registration is served
/// on whichever host the application is deployed on.
fn placeholder(value: &str, registration: &RelyingPartyRegistration, base_url: &str) -> String {
    value
        .replace("{baseUrl}", base_url)
        .replace("{registrationId}", registration.registration_id())
}

/// Returns the base64 body of a certificate.
///
/// A certificate is usually held in PEM form, so the armor lines and the line
/// breaks are removed; a value that is already a bare body is returned as it is,
/// minus any whitespace that would end up inside the XML element.
fn certificate_body(certificate: &str) -> String {
    certificate
        .lines()
        .filter(|line| {
            let line = line.trim();
            !line.is_empty() && !line.starts_with("-----")
        })
        .flat_map(|line| line.chars())
        .filter(|character| !character.is_whitespace())
        .collect()
}

/// Escapes the characters that would otherwise be read as XML markup.
fn escape_xml(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());

    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            other => escaped.push(other),
        }
    }

    escaped
}

#[cfg(test)]
mod tests {
    use crate::saml2::registration::{RelyingPartyRegistration, Saml2MessageBinding};
    use crate::saml2::metadata::Saml2Metadata;

    /// Returns the registration the assertions are made against.
    fn registration() -> RelyingPartyRegistration {
        RelyingPartyRegistration::with_registration_id("registration-id")
            .entity_id("rp-entity-id")
            .assertion_consumer_service_location("https://rp.example.org/acs")
            .single_logout_service_location("https://rp.example.org/logout/saml2/request")
            .single_logout_service_response_location(
                "https://rp.example.org/logout/saml2/response",
            )
            .signing_certificate(
                "-----BEGIN CERTIFICATE-----\n\
                 MIICgTCCAeoCCQCuVzyqFgMSyDANBgkqhkiG9w0BAQsFADCBhDELMAkGA1UEBh\n\
                 -----END CERTIFICATE-----\n",
            )
            .build()
    }

    #[test]
    fn the_entity_descriptor_carries_the_entity_id_and_the_consumer_service() {
        let metadata = Saml2Metadata::default().resolve(&registration(), "https://sp.example.org");

        assert!(
            metadata.contains(
                r#"<md:EntityDescriptor xmlns:md="urn:oasis:names:tc:SAML:2.0:metadata" entityID="rp-entity-id">"#
            ),
            "metadata was: {metadata}"
        );
        assert!(
            metadata.contains(
                r#"<md:AssertionConsumerService Binding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST" Location="https://rp.example.org/acs" index="1"/>"#
            ),
            "metadata was: {metadata}"
        );
        assert!(
            metadata.contains(
                r#"<md:SPSSODescriptor protocolSupportEnumeration="urn:oasis:names:tc:SAML:2.0:protocol">"#
            ),
            "metadata was: {metadata}"
        );
    }

    #[test]
    fn the_base_url_and_registration_id_placeholders_are_resolved() {
        let registration = RelyingPartyRegistration::with_registration_id("okta").build();

        let metadata =
            Saml2Metadata::default().resolve(&registration, "http://localhost:8080");

        assert!(
            metadata.contains(
                r#"entityID="http://localhost:8080/saml2/service-provider-metadata/okta""#
            ),
            "metadata was: {metadata}"
        );
        assert!(
            metadata.contains(
                r#"Location="http://localhost:8080/login/saml2/sso/okta" index="1""#
            ),
            "metadata was: {metadata}"
        );
    }

    #[test]
    fn a_registration_without_a_logout_location_publishes_no_logout_service() {
        let registration = RelyingPartyRegistration::with_registration_id("okta")
            .entity_id("rp-entity-id")
            .build();

        let metadata =
            Saml2Metadata::default().resolve(&registration, "https://sp.example.org");

        assert!(!metadata.contains("SingleLogoutService"), "metadata was: {metadata}");
        assert!(!metadata.contains("ResponseLocation"), "metadata was: {metadata}");
    }

    #[test]
    fn the_logout_service_carries_the_binding_and_the_response_location() {
        let metadata = Saml2Metadata::default().resolve(&registration(), "https://sp.example.org");

        assert!(
            metadata.contains(
                r#"<md:SingleLogoutService Binding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST" Location="https://rp.example.org/logout/saml2/request" ResponseLocation="https://rp.example.org/logout/saml2/response"/>"#
            ),
            "metadata was: {metadata}"
        );
    }

    #[test]
    fn a_redirect_binding_is_published_as_the_redirect_urn() {
        let registration = RelyingPartyRegistration::with_registration_id("okta")
            .entity_id("rp-entity-id")
            .assertion_consumer_service_location("https://rp.example.org/acs")
            .assertion_consumer_service_binding(Saml2MessageBinding::Redirect)
            .build();

        let metadata =
            Saml2Metadata::default().resolve(&registration, "https://sp.example.org");

        assert!(
            metadata.contains(
                r#"Binding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-Redirect" Location="https://rp.example.org/acs" index="1""#
            ),
            "metadata was: {metadata}"
        );
    }

    #[test]
    fn the_signing_certificate_becomes_a_single_line_key_descriptor() {
        let metadata = Saml2Metadata::default().resolve(&registration(), "https://sp.example.org");

        assert!(
            metadata.contains(r#"<md:KeyDescriptor use="signing">"#),
            "metadata was: {metadata}"
        );
        assert!(
            metadata.contains(
                "<ds:X509Certificate>MIICgTCCAeoCCQCuVzyqFgMSyDANBgkqhkiG9w0BAQsFADCBhDELMAkGA1UEBh</ds:X509Certificate>"
            ),
            "the certificate armor and newlines are gone, metadata was: {metadata}"
        );
    }

    #[test]
    fn a_registration_without_credentials_publishes_no_key_descriptor() {
        let registration = RelyingPartyRegistration::with_registration_id("okta")
            .entity_id("rp-entity-id")
            .build();

        let metadata =
            Saml2Metadata::default().resolve(&registration, "https://sp.example.org");

        assert!(!metadata.contains("KeyDescriptor"), "metadata was: {metadata}");
        assert!(!metadata.contains("X509Certificate"), "metadata was: {metadata}");
    }

    #[test]
    fn the_name_id_format_is_published_as_text() {
        let registration = RelyingPartyRegistration::with_registration_id("okta")
            .entity_id("rp-entity-id")
            .name_id_format("urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress")
            .build();

        let metadata =
            Saml2Metadata::default().resolve(&registration, "https://sp.example.org");

        assert!(
            metadata.contains(
                "<md:NameIDFormat>urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress</md:NameIDFormat>"
            ),
            "metadata was: {metadata}"
        );
    }

    #[test]
    fn a_single_registration_is_not_wrapped_in_an_entities_descriptor() {
        let metadata = Saml2Metadata::default().resolve_all(
            &[registration()],
            "https://sp.example.org",
        );

        assert!(metadata.contains("<md:EntityDescriptor"), "metadata was: {metadata}");
        assert!(!metadata.contains("<md:EntitiesDescriptor"), "metadata was: {metadata}");
    }

    #[test]
    fn several_registrations_are_wrapped_in_an_entities_descriptor() {
        let second = RelyingPartyRegistration::with_registration_id("okta")
            .entity_id("two")
            .build();

        let metadata = Saml2Metadata::default().resolve_all(
            &[registration(), second],
            "https://sp.example.org",
        );

        assert!(
            metadata.contains(
                r#"<md:EntitiesDescriptor xmlns:md="urn:oasis:names:tc:SAML:2.0:metadata">"#
            ),
            "metadata was: {metadata}"
        );
        assert!(
            metadata.contains(r#"entityID="rp-entity-id""#),
            "metadata was: {metadata}"
        );
        assert!(metadata.contains(r#"entityID="two""#), "metadata was: {metadata}");
    }

    #[test]
    fn attribute_values_are_escaped() {
        let registration = RelyingPartyRegistration::with_registration_id("okta")
            .entity_id(r#"rp-entity-id&"quoted""#)
            .build();

        let metadata =
            Saml2Metadata::default().resolve(&registration, "https://sp.example.org");

        assert!(
            metadata.contains(r#"entityID="rp-entity-id&amp;&quot;quoted&quot;""#),
            "metadata was: {metadata}"
        );
    }
}
