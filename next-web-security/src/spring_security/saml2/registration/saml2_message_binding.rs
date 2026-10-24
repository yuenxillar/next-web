/// The SAML 2.0 binding a message is exchanged over.
///
/// The binding is what an asserting party is told in the metadata: the
/// `Binding` attribute of the assertion consumer service and of the single
/// logout service.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Saml2MessageBinding {
    /// The message is posted as an HTML form.
    #[default]
    Post,
    /// The message is carried in the query string of a redirect.
    Redirect,
}

impl Saml2MessageBinding {
    /// Returns the identifier the binding is named by in a metadata document.
    pub fn urn(&self) -> &'static str {
        match self {
            Self::Post => "urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST",
            Self::Redirect => "urn:oasis:names:tc:SAML:2.0:bindings:HTTP-Redirect",
        }
    }
}
