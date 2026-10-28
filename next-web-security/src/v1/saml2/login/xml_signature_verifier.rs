//! The verifier that checks the signature of a response.

use crate::saml2::{
    error::{Saml2Error, Saml2ErrorKind},
    login::{Saml2Response, SignatureVerifier},
    registration::RelyingPartyRegistration,
    xml_dsig,
};

/// Verifies the signature of a response with the key the registration holds.
///
/// The response has to carry a signature, and the signature has to cover the
/// element it names: the digest of that element is checked against the one the
/// signature carries, and the signature itself is checked against
/// [`RelyingPartyRegistration::verification_public_key`]. Only the algorithms of a
/// SAML 2.0 profile are accepted — RSA-SHA256 for the signature, SHA-256 for the
/// digest, exclusive canonicalization — and a document that names any other is
/// refused.
///
/// A registration without a key cannot verify anything, so every response of such
/// a registration is refused rather than believed.
#[derive(Clone, Copy, Debug, Default)]
pub struct XmlSignatureVerifier;

impl SignatureVerifier for XmlSignatureVerifier {
    fn verify(
        &self,
        registration: &RelyingPartyRegistration,
        response: &Saml2Response,
    ) -> Result<(), Saml2Error> {
        if !response.is_signed() {
            return Err(Saml2Error::new(
                Saml2ErrorKind::VerificationFailed,
                "the response carries no signature",
            ));
        }
        let public_key = registration.verification_public_key().ok_or_else(|| {
            Saml2Error::new(
                Saml2ErrorKind::VerificationFailed,
                "the registration holds no key of the asserting party, so no response can be \
                 verified",
            )
        })?;

        xml_dsig::verify_signature(response.document(), public_key)
    }
}

#[cfg(test)]
mod tests {
    use base64::{Engine, engine::general_purpose::STANDARD};
    use rsa::{
        RsaPrivateKey, RsaPublicKey,
        pkcs1v15::SigningKey,
        pkcs8::{EncodePublicKey, LineEnding},
        signature::{SignatureEncoding, Signer},
    };
    use sha2::{Digest, Sha256};

    use super::*;
    use crate::saml2::registration::RelyingPartyRegistration;

    /// The namespace of the assertion.
    const SAML2: &str = "urn:oasis:names:tc:SAML:2.0:assertion";
    /// The namespace of the protocol.
    const SAML2P: &str = "urn:oasis:names:tc:SAML:2.0:protocol";
    /// The namespace of the signature.
    const DSIG: &str = "http://www.w3.org/2000/09/xmldsig#";

    /// Returns a key the tests sign with.
    fn key() -> RsaPrivateKey {
        RsaPrivateKey::new(&mut rsa::rand_core::OsRng, 2048).expect("a key is generated")
    }

    /// Returns the public key of a private key, in PEM form.
    fn public_key_pem(key: &RsaPrivateKey) -> String {
        RsaPublicKey::from(key)
            .to_public_key_pem(LineEnding::LF)
            .expect("the key is written")
    }

    /// Returns the registration of a response signed with the given key.
    fn registration(key: &RsaPrivateKey) -> RelyingPartyRegistration {
        RelyingPartyRegistration::with_registration_id("okta")
            .entity_id("sp-entity-id")
            .assertion_consumer_service_location("https://sp.example.org/login/saml2/sso/okta")
            .asserting_party_entity_id("idp-entity-id")
            .verification_public_key(public_key_pem(key))
            .build()
    }

    /// Returns the assertion of the tests, without a signature.
    fn assertion() -> String {
        format!(
            r#"<saml2:Assertion xmlns:saml2="{SAML2}" ID="the-assertion" IssueInstant="2026-10-19T13:00:00Z" Version="2.0"><saml2:Issuer>idp-entity-id</saml2:Issuer><saml2:Subject><saml2:NameID>Max</saml2:NameID></saml2:Subject></saml2:Assertion>"#
        )
    }

    /// Returns a response whose assertion is signed with the given key.
    ///
    /// The digest is taken over the canonical form of the assertion without the
    /// signature, and the signature over the canonical form of `SignedInfo`, which
    /// is what a SAML 2.0 profile prescribes. The document is built in the
    /// canonical form itself, so a verifier that canonicalizes it reaches the same
    /// bytes.
    fn signed_response(key: &RsaPrivateKey) -> String {
        let assertion = assertion();
        let signature = signature_of(&assertion, key);

        format!(
            r#"<saml2p:Response xmlns:saml2p="{SAML2P}" ID="the-response" IssueInstant="2026-10-19T13:00:00Z" Version="2.0">{assertion}{signature}</saml2p:Response>"#
        )
    }

    /// Returns the signature element of the given assertion.
    fn signature_of(assertion: &str, key: &RsaPrivateKey) -> String {
        let signature_element_of = |digest: &str| {
            format!(
                r##"<ds:Signature xmlns:ds="{DSIG}"><ds:SignedInfo><ds:CanonicalizationMethod Algorithm="http://www.w3.org/2001/10/xml-exc-c14n#"></ds:CanonicalizationMethod><ds:SignatureMethod Algorithm="http://www.w3.org/2001/04/xmldsig-more#rsa-sha256"></ds:SignatureMethod><ds:Reference URI="#the-assertion"><ds:Transforms><ds:Transform Algorithm="http://www.w3.org/2000/09/xmldsig#enveloped-signature"></ds:Transform><ds:Transform Algorithm="http://www.w3.org/2001/10/xml-exc-c14n#"></ds:Transform></ds:Transforms><ds:DigestMethod Algorithm="http://www.w3.org/2001/04/xmlenc#sha256"></ds:DigestMethod><ds:DigestValue>{digest}</ds:DigestValue></ds:Reference></ds:SignedInfo><ds:SignatureValue></ds:SignatureValue></ds:Signature>"##
            )
        };

        // The digest covers the assertion without the signature, which is the
        // form the verifier canonicalizes.
        let digest = STANDARD.encode(Sha256::digest(assertion.as_bytes()));
        let unsigned = signature_element_of(&digest);
        let signed_info = signed_info_of(&unsigned);
        let signature = SigningKey::<Sha256>::new(key.clone()).sign(signed_info.as_bytes());
        let signature_value = STANDARD.encode(signature.to_bytes());

        signature_element_of(&digest).replace(
            "<ds:SignatureValue></ds:SignatureValue>",
            &format!("<ds:SignatureValue>{signature_value}</ds:SignatureValue>"),
        )
    }

    /// Returns the `SignedInfo` element of a signature, in the form the signature
    /// covers: the canonical form declares the namespace it uses itself, because
    /// canonicalization writes a declaration where the prefix is first used.
    fn signed_info_of(signature: &str) -> String {
        let start = signature
            .find("<ds:SignedInfo>")
            .expect("the signature has a SignedInfo");
        let end = signature
            .find("</ds:SignedInfo>")
            .expect("the signature has a SignedInfo")
            + "</ds:SignedInfo>".len();

        signature[start..end].replacen(
            "<ds:SignedInfo>",
            &format!(r##"<ds:SignedInfo xmlns:ds="{DSIG}">"##),
            1,
        )
    }

    #[test]
    fn the_signature_of_a_response_is_verified() {
        let key = key();
        let response = Saml2Response::parse(&signed_response(&key)).expect("the response parses");

        XmlSignatureVerifier
            .verify(&registration(&key), &response)
            .expect("the signature of the response holds");
    }

    #[test]
    fn a_response_whose_assertion_changed_is_refused() {
        let key = key();
        let signed = signed_response(&key);
        let tampered = signed.replace("<saml2:NameID>Max</saml2:NameID>", "<saml2:NameID>Eve</saml2:NameID>");
        let response = Saml2Response::parse(&tampered).expect("the response parses");

        let error = XmlSignatureVerifier
            .verify(&registration(&key), &response)
            .expect_err("the signature does not cover the changed assertion");

        assert_eq!(error.kind(), Saml2ErrorKind::VerificationFailed);
    }

    #[test]
    fn a_response_signed_with_another_key_is_refused() {
        let signer = key();
        let other = key();
        let response = Saml2Response::parse(&signed_response(&signer)).expect("the response parses");

        let error = XmlSignatureVerifier
            .verify(&registration(&other), &response)
            .expect_err("the signature is not the one of the key");

        assert_eq!(error.kind(), Saml2ErrorKind::VerificationFailed);
    }

    #[test]
    fn a_response_without_a_signature_is_refused() {
        let key = key();
        let response = Saml2Response::parse(&format!(
            r#"<saml2p:Response xmlns:saml2p="{SAML2P}" ID="the-response" Version="2.0">{}</saml2p:Response>"#,
            assertion()
        ))
        .expect("the response parses");

        let error = XmlSignatureVerifier
            .verify(&registration(&key), &response)
            .expect_err("an unsigned response is not believed");

        assert_eq!(error.kind(), Saml2ErrorKind::VerificationFailed);
    }

    #[test]
    fn a_registration_without_a_key_is_refused() {
        let key = key();
        let response = Saml2Response::parse(&signed_response(&key)).expect("the response parses");
        let registration = RelyingPartyRegistration::with_registration_id("okta").build();

        let error = XmlSignatureVerifier
            .verify(&registration, &response)
            .expect_err("a registration without a key cannot verify a response");

        assert_eq!(error.kind(), Saml2ErrorKind::VerificationFailed);
    }

    #[test]
    fn a_sha1_signature_is_refused() {
        let key = key();
        let signed = signed_response(&key);
        let sha1 = signed.replace(
            "http://www.w3.org/2001/04/xmldsig-more#rsa-sha256",
            "http://www.w3.org/2000/09/xmldsig#rsa-sha1",
        );
        let response = Saml2Response::parse(&sha1).expect("the response parses");

        let error = XmlSignatureVerifier
            .verify(&registration(&key), &response)
            .expect_err("the profile does not use SHA-1");

        assert_eq!(error.kind(), Saml2ErrorKind::VerificationFailed);
    }
}
