use crate::saml2::{
    error::{Saml2Error, Saml2ErrorKind},
    login::Saml2Response,
    registration::RelyingPartyRegistration,
};

/// Decides whether a response may be believed.
///
/// A response only proves who the user is when its signature covers what it
/// claims, and when its conditions, audience, recipient and in-response-to
/// identifier hold for this service provider. Those checks are what an
/// implementation does; the service provider does not accept a response without
/// one.
pub trait AssertionVerifier: Send + Sync {
    /// Returns `Ok(())` when the response may be believed.
    ///
    /// # Errors
    ///
    /// Returns [`Saml2ErrorKind::VerificationFailed`] when the response is not
    /// acceptable, with a message that names the check that failed.
    fn verify(
        &self,
        registration: &RelyingPartyRegistration,
        response: &Saml2Response,
    ) -> Result<(), Saml2Error>;
}

/// A verifier that believes no response.
///
/// It is what an application configures when it wants a login endpoint that can
/// never sign anyone in, or as a stand-in while a verifier of its own is written.
/// The configuration of a login ships a verifier that checks the signature of a
/// response instead ([`XmlSignatureVerifier`]: super::XmlSignatureVerifier), so
/// this one is opt-in.
#[derive(Clone, Copy, Debug, Default)]
pub struct RejectingAssertionVerifier;

impl AssertionVerifier for RejectingAssertionVerifier {
    fn verify(
        &self,
        _registration: &RelyingPartyRegistration,
        _response: &Saml2Response,
    ) -> Result<(), Saml2Error> {
        Err(Saml2Error::new(
            Saml2ErrorKind::VerificationFailed,
            "no assertion verifier is configured, so the response is not believed; supply one \
             that verifies the signature of a response",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::saml2::registration::RelyingPartyRegistration;
    use crate::saml2::Saml2ErrorKind;

    /// Returns a registration configured for an identity provider.
    fn registration() -> RelyingPartyRegistration {
        RelyingPartyRegistration::with_registration_id("okta")
            .entity_id("sp-entity-id")
            .asserting_party_entity_id("idp-entity-id")
            .build()
    }

    #[test]
    fn the_default_verifier_rejects_every_response() {
        let response = Saml2Response::parse(
            r#"<saml2p:Response xmlns:saml2p="urn:oasis:names:tc:SAML:2.0:protocol"><saml2:Issuer xmlns:saml2="urn:oasis:names:tc:SAML:2.0:assertion">idp-entity-id</saml2:Issuer></saml2p:Response>"#,
        )
        .expect("the response parses");

        let error = RejectingAssertionVerifier
            .verify(&registration(), &response)
            .expect_err("no verifier is configured");

        assert_eq!(error.kind(), Saml2ErrorKind::VerificationFailed);
        assert!(error.message().contains("no assertion verifier"));
    }
}
