//! The CBOR fixtures the Web Authentication tests are built with.
//!
//! A ceremony is a pile of encodings, and a test that wants to reach one check
//! has to say what all the others encode. The helpers here write CBOR by hand
//! rather than with the decoder under test, so a fixture is data the test states
//! rather than something the test derives from the code it checks.
//!
//! The keys of the fixtures are fixed bytes rather than generated ones: a test
//! that fails twice must fail the same way.

use base64::Engine;

/// Decodes a base64url string the way the browser and the RFCs write one.
pub(crate) fn base64_url(value: &str) -> Vec<u8> {
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(value)
        .expect("the value is base64url")
}

/// Encodes a base64url string, the way a browser submits a byte string.
pub(crate) fn to_base64_url(value: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(value)
}

/// Encodes a CBOR unsigned or negative integer.
pub(crate) fn cbor_integer(value: i64) -> Vec<u8> {
    if value >= 0 {
        cbor_head(0, value as u64)
    } else {
        cbor_head(1, (-1 - value) as u64)
    }
}

/// Encodes a CBOR byte string.
pub(crate) fn cbor_bytes(value: &[u8]) -> Vec<u8> {
    let mut encoded = cbor_head(2, value.len() as u64);
    encoded.extend_from_slice(value);

    encoded
}

/// Encodes a CBOR text string.
pub(crate) fn cbor_text(value: &str) -> Vec<u8> {
    let mut encoded = cbor_head(3, value.len() as u64);
    encoded.extend_from_slice(value.as_bytes());

    encoded
}

/// Encodes the head of a CBOR item: its major type and its length.
pub(crate) fn cbor_head(major: u8, length: u64) -> Vec<u8> {
    match length {
        0..=23 => vec![(major << 5) | length as u8],
        24..=255 => vec![(major << 5) | 24, length as u8],
        256..=65535 => {
            let mut encoded = vec![(major << 5) | 25];
            encoded.extend_from_slice(&(length as u16).to_be_bytes());
            encoded
        }
        _ => {
            let mut encoded = vec![(major << 5) | 26];
            encoded.extend_from_slice(&(length as u32).to_be_bytes());
            encoded
        }
    }
}

/// Encodes a CBOR map of the given pairs, in the order they are given.
pub(crate) fn cbor_map(pairs: &[(Vec<u8>, Vec<u8>)]) -> Vec<u8> {
    let mut encoded = cbor_head(5, pairs.len() as u64);
    for (label, value) in pairs {
        encoded.extend_from_slice(label);
        encoded.extend_from_slice(value);
    }

    encoded
}

// ---------------------------------------------------------------------------
// The vectors of RFC 7515, which are the only signatures here that another
// implementation produced: the test suites of this port sign with a key the
// test holds, which proves the plumbing and not the arithmetic.
// ---------------------------------------------------------------------------

/// The `x` coordinate of the P-256 key of RFC 7515 appendix A.3.
pub(crate) const RFC_7515_A3_X: &str = "f83OJ3D2xF1Bg8vub9tLe1gHMzV76e8Tus9uPHvRVEU";

/// The `y` coordinate of the P-256 key of RFC 7515 appendix A.3.
pub(crate) const RFC_7515_A3_Y: &str = "x_FEzRu9m36HLN_tue659LNpXW6pCyStikYjKIWI5a0";

/// The signing input RFC 7515 appendix A.3 signs.
pub(crate) const RFC_7515_A3_SIGNING_INPUT: &str = "eyJhbGciOiJFUzI1NiJ9.eyJpc3MiOiJqb2UiLA0KICJleHAiOjEzMDA4MTkzODAsDQogImh0dHA6Ly9leGFtcGxlLmNvbS9pc19yb290Ijp0cnVlfQ";

/// The signature of RFC 7515 appendix A.3, over the signing input above.
pub(crate) const RFC_7515_A3_SIGNATURE: &str = "DtEhU3ljbEg8L38VWAfUAqOyKAM6-Xx-F4GawxaepmXFCgfTjDxw5djxLa8ISlSApmWQxfKTUJqPP3-Kg6NU1Q";

/// The signing input RFC 7515 appendix A.2 signs.
pub(crate) const RFC_7515_A2_SIGNING_INPUT: &str = "eyJhbGciOiJSUzI1NiJ9.eyJpc3MiOiJqb2UiLA0KICJleHAiOjEzMDA4MTkzODAsDQogImh0dHA6Ly9leGFtcGxlLmNvbS9pc19yb290Ijp0cnVlfQ";

/// The modulus of the RSA key of RFC 7515 appendix A.2.
pub(crate) const RFC_7515_A2_MODULUS: &str = "ofgWCuLjybRlzo0tZWJjNiuSfb4p4fAkd_wWJcyQoTbji9k0l8W26mPddxHmfHQp-Vaw-4qPCJrcS2mJPMEzP1Pt0Bm4d4QlL-yRT-SFd2lZS-pCgNMsD1W_YpRPEwOWvG6b32690r2jZ47soMZo9wGzjb_7OMg0LOL-bSf63kpaSHSXndS5z5rexMdbBYUsLA9e-KXBdQOS-UTo7WTBEMa2R2CapHg665xsmtdVMTBQY4uDZlxvb3qCo5ZwKh9kG4LT6_I5IhlJH7aGhyxXFvUK-DWNmoudF8NAco9_h9iaGNj8q2ethFkMLs91kzk2PAcDTW9gb54h4FRWyuXpoQ";

/// The signature of RFC 7515 appendix A.2, over the signing input above.
pub(crate) const RFC_7515_A2_SIGNATURE: &str = "cC4hiUPoj9Eetdgtv3hF80EGrhuB__dzERat0XF9g2VtQgr9PJbu3XOiZj5RZmh7AAuHIm4Bh-0Qc_lF5YKt_O8W2Fp5jujGbds9uJdbF9CUAr7t1dnZcAcQjbKBYNX4BAynRFdiuB--f_nZLgrnbyTyWzO75vRK5h6xBArLIARNPvkSjtQBMHlb1L07Qe7K0GarZRmB_eSN9383LcOLn6_dO--xi12jzDwusC-eOkHWEsqtFZESc6BfI7noOPqvhJ1phCnvWh6IeYI2w9QOYEUipUTI8np6LbgGY9Fs98rqVt5AXLIhWkWywlVmtVrBp0igcN_IoypGlUPQGe77Rw";

/// The COSE key of RFC 7515 appendix A.3, the way an authenticator with a
/// P-256 credential writes it.
pub(crate) fn es256_cose_key() -> Vec<u8> {
    ec2_cose_key(-7, 1, &base64_url(RFC_7515_A3_X), &base64_url(RFC_7515_A3_Y))
}

/// The COSE key of RFC 7515 appendix A.2, the way an authenticator with an RSA
/// credential writes it.
pub(crate) fn rs256_cose_key() -> Vec<u8> {
    cbor_map(&[
        (cbor_integer(1), cbor_integer(3)),
        (cbor_integer(3), cbor_integer(-257)),
        (cbor_integer(-1), cbor_bytes(&base64_url(RFC_7515_A2_MODULUS))),
        (cbor_integer(-2), cbor_bytes(&[0x01, 0x00, 0x01])),
    ])
}

/// Encodes a COSE key of an elliptic curve credential.
pub(crate) fn ec2_cose_key(algorithm: i64, curve: i64, x: &[u8], y: &[u8]) -> Vec<u8> {
    cbor_map(&[
        (cbor_integer(1), cbor_integer(2)),
        (cbor_integer(3), cbor_integer(algorithm)),
        (cbor_integer(-1), cbor_integer(curve)),
        (cbor_integer(-2), cbor_bytes(x)),
        (cbor_integer(-3), cbor_bytes(y)),
    ])
}

/// Returns the bytes of the public key of a P-256 signing key, as an
/// authenticator would encode them.
pub(crate) fn cose_key_of(signing_key: &p256::ecdsa::SigningKey) -> Vec<u8> {
    // The SEC1 point of an uncompressed key is `0x04 || x || y`, and P-256
    // coordinates are 32 bytes each.
    let point = signing_key.verifying_key().to_sec1_point(false);
    let bytes = point.as_bytes();

    ec2_cose_key(-7, 1, &bytes[1..33], &bytes[33..65])
}

/// Signs a message the way an authenticator signs a ceremony.
pub(crate) fn sign(signing_key: &p256::ecdsa::SigningKey, message: &[u8]) -> Vec<u8> {
    use p256::ecdsa::signature::Signer;

    let signature: p256::ecdsa::Signature = signing_key.sign(message);

    signature.to_bytes().to_vec()
}

// ---------------------------------------------------------------------------
// The ceremony fixtures: one registration and one authentication, built out of
// the same fixed key so that a failure is the same failure twice.
// ---------------------------------------------------------------------------

/// The flag that says the user took part in the ceremony.
pub(crate) const USER_PRESENT: u8 = 0x01;

/// The flag that says the authenticator verified the user.
pub(crate) const USER_VERIFIED: u8 = 0x04;

/// The flag that says the attested credential data follows the header.
pub(crate) const ATTESTED_CREDENTIAL_DATA: u8 = 0x40;

/// Returns the signing key the fixtures of the ceremonies use.
pub(crate) fn signing_key() -> p256::ecdsa::SigningKey {
    p256::ecdsa::SigningKey::from_slice(&[0x11; 32]).expect("the seed is a valid P-256 scalar")
}

/// Returns the hash of a relying party id, which the authenticator data carries.
pub(crate) fn rp_id_hash(rp_id: &str) -> Vec<u8> {
    use sha2::{Digest, Sha256};

    Sha256::digest(rp_id.as_bytes()).to_vec()
}

/// Returns authenticator data with the given relying party, flags and counter,
/// and, when one is given, the attested credential of a registration.
pub(crate) fn authenticator_data(
    rp_id: &str,
    flags: u8,
    sign_count: u32,
    credential: Option<(&[u8], &[u8])>,
) -> Vec<u8> {
    let mut raw = rp_id_hash(rp_id);
    raw.push(flags);
    raw.extend_from_slice(&sign_count.to_be_bytes());
    if let Some((credential_id, public_key)) = credential {
        raw.extend_from_slice(&[0x11; 16]);
        raw.extend_from_slice(&(credential_id.len() as u16).to_be_bytes());
        raw.extend_from_slice(credential_id);
        raw.extend_from_slice(public_key);
    }

    raw
}

/// Returns an attestation object of the given format and statement.
pub(crate) fn attestation_object(
    format: &str,
    statement: &[(Vec<u8>, Vec<u8>)],
    authenticator_data: &[u8],
) -> Vec<u8> {
    cbor_map(&[
        (cbor_text("fmt"), cbor_text(format)),
        (cbor_text("attStmt"), cbor_map(statement)),
        (cbor_text("authData"), cbor_bytes(authenticator_data)),
    ])
}

/// Returns the raw bytes of the client data of a ceremony.
pub(crate) fn client_data_json(
    kind: &str,
    challenge: &[u8],
    origin: &str,
    cross_origin: bool,
) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "type": kind,
        "challenge": to_base64_url(challenge),
        "origin": origin,
        "crossOrigin": cross_origin,
    }))
    .expect("the client data serializes")
}

/// Returns the JSON a browser submits to register a credential.
pub(crate) fn registration_submission(
    credential_id: &[u8],
    client_data_json: &[u8],
    attestation_object: &[u8],
) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "id": to_base64_url(credential_id),
        "rawId": to_base64_url(credential_id),
        "type": "public-key",
        "response": {
            "clientDataJSON": to_base64_url(client_data_json),
            "attestationObject": to_base64_url(attestation_object),
            "transports": ["internal"],
        },
    }))
    .expect("the submission serializes")
}

/// The origin every ceremony of the tests runs on.
pub(crate) const ORIGIN: &str = "https://sp.example.org";

/// The identifier of the relying party of the tests.
pub(crate) const RP_ID: &str = "sp.example.org";

/// Registers `the-credential` for a user through the manager, and returns it.
pub(crate) fn register_credential(
    manager: &crate::webauthn::WebAuthnManager,
    credential_id: &[u8],
    user_name: &str,
) -> crate::webauthn::Credential {
    let key = signing_key();
    let options = manager.create_registration_options(user_name, user_name);
    let cose_key = cose_key_of(&key);
    let authenticator_data = authenticator_data(
        RP_ID,
        USER_PRESENT | ATTESTED_CREDENTIAL_DATA,
        0,
        Some((credential_id, &cose_key)),
    );
    let submission = registration_submission(
        credential_id,
        &client_data_json(crate::webauthn::CREATE, options.challenge(), ORIGIN, false),
        &attestation_object("none", &[], &authenticator_data),
    );
    let request =
        crate::webauthn::RegistrationRequest::parse(&submission).expect("the request parses");

    manager
        .register(user_name, &request)
        .expect("the registration holds")
}

/// Returns the browser submission of an authentication of a registered
/// credential, signed with the key the fixtures register.
pub(crate) fn authenticate_submission(
    manager: &crate::webauthn::WebAuthnManager,
    credential_id: &[u8],
    user_name: &str,
    sign_count: u32,
) -> Vec<u8> {
    let key = signing_key();
    let options = manager.create_authentication_options(user_name);
    let client_data = client_data_json(crate::webauthn::GET, options.challenge(), ORIGIN, false);
    let authenticator_data = authenticator_data(RP_ID, USER_PRESENT, sign_count, None);
    let mut message = authenticator_data.clone();
    message.extend_from_slice(&sha256(&client_data));

    authentication_submission(
        credential_id,
        &client_data,
        &authenticator_data,
        &sign(&key, &message),
        None,
    )
}

/// Returns the SHA-256 of a message, which a ceremony signs over.
pub(crate) fn sha256(message: &[u8]) -> Vec<u8> {
    use sha2::{Digest, Sha256};

    Sha256::digest(message).to_vec()
}

/// Returns the JSON a browser submits to authenticate.
pub(crate) fn authentication_submission(
    credential_id: &[u8],
    client_data_json: &[u8],
    authenticator_data: &[u8],
    signature: &[u8],
    user_handle: Option<&[u8]>,
) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "id": to_base64_url(credential_id),
        "rawId": to_base64_url(credential_id),
        "type": "public-key",
        "response": {
            "clientDataJSON": to_base64_url(client_data_json),
            "authenticatorData": to_base64_url(authenticator_data),
            "signature": to_base64_url(signature),
            "userHandle": user_handle.map(to_base64_url),
        },
    }))
    .expect("the submission serializes")
}

