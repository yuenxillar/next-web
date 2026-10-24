//! A response signed by another XML signature implementation.
//!
//! The fixture is a SAML 2.0 response whose assertion was signed by
//! `System.Security.Cryptography.Xml.SignedXml` of .NET: the signature is over
//! the exclusive canonical form of the assertion, with the enveloped-signature
//! transform, RSA-SHA256 and a SHA-256 digest. The document was signed as it is —
//! pretty-printed, with the namespaces declared on the root — so the test proves
//! that this crate canonicalizes a document the way another implementation does,
//! which is what a login against a real asserting party depends on.
//!
//! `idp-key-parameters.txt` holds the modulus and the exponent of the public key
//! of the fixture, and the test builds the key from them.

use base64::{Engine, engine::general_purpose::STANDARD};
use next_web_security::saml2::{
    login::{Saml2Response, SignatureVerifier, XmlSignatureVerifier},
    registration::RelyingPartyRegistration,
    Saml2ErrorKind,
};
use rsa::{
    BigUint, RsaPublicKey,
    pkcs1::EncodeRsaPublicKey,
    pkcs8::LineEnding,
};

/// The signed response of the fixture.
const RESPONSE: &str = include_str!("fixtures/saml2/response-signed.xml");

/// The parameters of the key of the asserting party of the fixture.
const KEY_PARAMETERS: &str = include_str!("fixtures/saml2/idp-key-parameters.txt");

/// Returns the public key of the asserting party of the fixture, in PEM form.
fn public_key_pem() -> String {
    let parameter = |name: &str| {
        KEY_PARAMETERS
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{name}=")))
            .unwrap_or_else(|| panic!("the fixture names the {name} of the key"))
    };
    let modulus = STANDARD
        .decode(parameter("modulus"))
        .expect("the modulus is base64");
    let exponent = STANDARD
        .decode(parameter("exponent"))
        .expect("the exponent is base64");
    let key = RsaPublicKey::new(
        BigUint::from_bytes_be(&modulus),
        BigUint::from_bytes_be(&exponent),
    )
    .expect("the parameters form a key");

    key.to_pkcs1_pem(LineEnding::LF)
        .expect("the key is written")
        .to_string()
}

/// Returns the registration the fixture is submitted for.
fn registration() -> RelyingPartyRegistration {
    RelyingPartyRegistration::with_registration_id("okta")
        .entity_id("sp-entity-id")
        .assertion_consumer_service_location("https://sp.example.org/login/saml2/sso/okta")
        .asserting_party_entity_id("https://idp.example.org")
        .verification_public_key(public_key_pem())
        .build()
}

#[test]
fn the_signature_of_a_response_signed_by_another_implementation_is_verified() {
    let response = Saml2Response::parse(RESPONSE).expect("the fixture parses");

    assert_eq!(response.name_id(), Some("Max"), "the fixture names a user");
    XmlSignatureVerifier
        .verify(&registration(), &response)
        .expect("the signature of the fixture holds");
}

#[test]
fn a_response_of_the_other_implementation_that_changed_is_refused() {
    let tampered = RESPONSE.replace(">Max<", ">Eve<");
    let response = Saml2Response::parse(&tampered).expect("the tampered fixture parses");

    let error = XmlSignatureVerifier
        .verify(&registration(), &response)
        .expect_err("the signature does not cover the changed name");

    assert_eq!(error.kind(), Saml2ErrorKind::VerificationFailed);
}

#[test]
fn a_response_of_the_other_implementation_is_refused_for_another_key() {
    let response = Saml2Response::parse(RESPONSE).expect("the fixture parses");
    let other = RsaPublicKey::from(
        &rsa::RsaPrivateKey::new(&mut rsa::rand_core::OsRng, 2048)
            .expect("another key is generated"),
    );
    let registration = RelyingPartyRegistration::with_registration_id("okta")
        .entity_id("sp-entity-id")
        .verification_public_key(
            other
                .to_pkcs1_pem(LineEnding::LF)
                .expect("the other key is written")
                .to_string(),
        )
        .build();

    let error = XmlSignatureVerifier
        .verify(&registration, &response)
        .expect_err("the signature is not the one of the key");

    assert_eq!(error.kind(), Saml2ErrorKind::VerificationFailed);
}
