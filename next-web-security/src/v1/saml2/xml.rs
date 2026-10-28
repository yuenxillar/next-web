//! The XML the SAML 2.0 messages are written as.

use crate::saml2::registration::RelyingPartyRegistration;

/// Replaces the placeholders a registration field may carry.
///
/// A field is a template, not an absolute URL: the same registration is served
/// on whichever host the application is deployed on.
pub(crate) fn resolve_placeholders(
    value: &str,
    registration: &RelyingPartyRegistration,
    base_url: &str,
) -> String {
    value
        .replace("{baseUrl}", base_url)
        .replace("{registrationId}", registration.registration_id())
}

/// Escapes the characters that would otherwise be read as XML markup.
pub(crate) fn escape_xml(value: &str) -> String {
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
