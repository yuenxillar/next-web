use std::collections::BTreeMap;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use rsa::pkcs1::{DecodeRsaPrivateKey, DecodeRsaPublicKey};
use rsa::pkcs1v15::{Signature as RsaSignature, SigningKey, VerifyingKey};
use rsa::pkcs8::{DecodePrivateKey, DecodePublicKey};
use rsa::signature::{SignatureEncoding, Signer, Verifier};
use rsa::{RsaPrivateKey, RsaPublicKey};
use sha2::Sha256;

/// Builds Alipay's canonical signing string by sorting parameters and skipping empty values.
pub fn build_sign_content(params: &BTreeMap<&'static str, String>) -> String {
    params
        .iter()
        .filter(|(key, value)| !value.is_empty() && **key != "sign")
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("&")
}

/// Signs content with RSA2.
pub fn sign_with_rsa2(content: &str, private_key_pem: &str) -> Result<String, String> {
    let private_key = decode_private_key(private_key_pem)
        .map_err(|e| format!("Failed to decode private key: {}", e))?;
    let signing_key = SigningKey::<Sha256>::new(private_key);
    let signature = signing_key.sign(content.as_bytes());
    Ok(STANDARD.encode(signature.to_bytes()))
}

/// Verifies a RSA2 signature.
pub fn verify_with_rsa2(content: &str, signature: &str, public_key: &str) -> Result<bool, String> {
    let public_key = decode_public_key(public_key)?;
    let verifying_key = VerifyingKey::<Sha256>::new(public_key);
    let signature = STANDARD
        .decode(signature)
        .map_err(|error| format!("Failed to decode base64 signature: {}", error))?;
    let signature = RsaSignature::try_from(signature.as_slice())
        .map_err(|error| format!("Failed to parse RSA signature: {}", error))?;

    Ok(verifying_key.verify(content.as_bytes(), &signature).is_ok())
}

fn decode_private_key(private_key: &str) -> Result<RsaPrivateKey, String> {
    let private_key = private_key.trim();

    if private_key.contains("BEGIN") {
        return RsaPrivateKey::from_pkcs8_pem(private_key)
            .or_else(|_| RsaPrivateKey::from_pkcs1_pem(private_key))
            .map_err(|error| format!("Failed to parse PEM private key: {}", error));
    }

    let pkcs8_pem = wrap_pem(private_key, "PRIVATE KEY");
    if let Ok(key) = RsaPrivateKey::from_pkcs8_pem(&pkcs8_pem) {
        return Ok(key);
    }

    let pkcs1_pem = wrap_pem(private_key, "RSA PRIVATE KEY");
    RsaPrivateKey::from_pkcs1_pem(&pkcs1_pem)
        .map_err(|error| format!("Failed to parse as PKCS#1 private key: {}", error))
}

fn decode_public_key(public_key: &str) -> Result<RsaPublicKey, String> {
    let public_key = public_key.trim();

    if public_key.contains("BEGIN") {
        return RsaPublicKey::from_public_key_pem(public_key)
            .or_else(|_| RsaPublicKey::from_pkcs1_pem(public_key))
            .map_err(|error| format!("Failed to parse PEM key: {}", error));
    }

    let spki_pem = wrap_pem(public_key, "PUBLIC KEY");
    if let Ok(key) = RsaPublicKey::from_public_key_pem(&spki_pem) {
        return Ok(key);
    }

    let pkcs1_pem = wrap_pem(public_key, "RSA PUBLIC KEY");
    RsaPublicKey::from_pkcs1_pem(&pkcs1_pem)
        .map_err(|error| format!("Failed to parse as PKCS#1 key: {}", error))
}

fn wrap_pem(body: &str, label: &str) -> String {
    let body = body
        .chars()
        .filter(|character| !character.is_ascii_whitespace())
        .collect::<String>();

    let mut pem = String::new();
    pem.push_str("-----BEGIN ");
    pem.push_str(label);
    pem.push_str("-----\n");

    for chunk in body.as_bytes().chunks(64) {
        pem.push_str(&String::from_utf8_lossy(chunk));
        pem.push('\n');
    }

    pem.push_str("-----END ");
    pem.push_str(label);
    pem.push_str("-----\n");
    pem
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use rand::thread_rng;
    use rsa::pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding};
    use rsa::{RsaPrivateKey, RsaPublicKey};

    use super::{build_sign_content, sign_with_rsa2, verify_with_rsa2};

    #[test]
    fn sign_content_is_sorted_and_skips_empty_values() {
        let mut params = BTreeMap::new();
        params.insert("b", "2".to_string());
        params.insert("a", "1".to_string());
        params.insert("sign", "ignored".to_string());
        params.insert("empty", String::new());
        params.insert("sign_type", "RSA2".to_string());

        let content = build_sign_content(&params);

        assert_eq!(content, "a=1&b=2&sign_type=RSA2");
    }

    #[test]
    fn rsa2_sign_and_verify_roundtrip() {
        let mut rng = thread_rng();
        let private_key = RsaPrivateKey::new(&mut rng, 2048).expect("private key");
        let public_key = RsaPublicKey::from(&private_key);
        let private_pem = private_key
            .to_pkcs8_pem(LineEnding::LF)
            .expect("private pem")
            .to_string();
        let public_pem = public_key
            .to_public_key_pem(LineEnding::LF)
            .expect("public pem");

        let signature = sign_with_rsa2("hello-alipay", &private_pem).expect("signature");
        let verified = verify_with_rsa2("hello-alipay", &signature, &public_pem).expect("verify");

        assert!(verified);
    }
}
