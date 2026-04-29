use std::{collections::BTreeMap, fs};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use rsa::pkcs1::{DecodeRsaPrivateKey, DecodeRsaPublicKey};
use rsa::pkcs1v15::{Signature as RsaSignature, SigningKey, VerifyingKey};
use rsa::pkcs8::{AssociatedOid, DecodePrivateKey, DecodePublicKey};
use rsa::signature::{SignatureEncoding, Signer, Verifier};
use rsa::{RsaPrivateKey, RsaPublicKey};
use sha2::{Digest, Sha256};

/// Builds Alipay's canonical signing string by sorting parameters and skipping empty values.
pub fn build_sign_content(params: &BTreeMap<&'static str, String>) -> String {
    build_sign_content_with_options(params, false)
}

/// Builds Alipay's notification signing string by skipping `sign` and `sign_type`.
pub fn build_notify_sign_content(params: &BTreeMap<&str, &str>) -> String {
    params
        .iter()
        .filter(|(key, value)| !value.is_empty() && **key != "sign" && **key != "sign_type")
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("&")
}

fn build_sign_content_with_options(
    params: &BTreeMap<&'static str, String>,
    skip_sign_type: bool,
) -> String {
    params
        .iter()
        .filter(|(key, value)| {
            !value.is_empty() && (!skip_sign_type || ( **key != "sign" && **key != "sign_type"))
        })
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

pub fn signing_key<D>(private_key_pem: impl AsRef<str>) -> Result<SigningKey<D>, String>
where
    D: Digest + AssociatedOid,
{
    let private_key = decode_private_key(private_key_pem.as_ref())
        .map_err(|e| format!("Failed to decode private key: {}", e))?;

    Ok(SigningKey::<D>::new(private_key))
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

/// Verifies a RSA2 signature with the public key contained in an X.509 certificate.
pub fn verify_with_rsa2_cert(
    content: &str,
    signature: &str,
    cert_path: impl AsRef<str>,
) -> Result<bool, String> {
    let public_key = public_key_from_cert_path(cert_path)?;
    verify_with_rsa2(content, signature, &public_key)
}

/// Calculates Alipay certificate SN: MD5(issuer + decimal serial number).
pub fn cert_sn_from_path(cert_path: impl AsRef<str>) -> Result<String, String> {
    let cert = read_certificate(cert_path.as_ref())?;
    let parsed = parse_certificate(&cert)?;
    Ok(format!(
        "{:x}",
        md5::compute(format!("{}{}", parsed.issuer, parsed.serial_decimal))
    ))
}

/// Calculates Alipay root certificate SN by joining RSA root certificate SNs with `_`.
pub fn root_cert_sn_from_path(cert_path: impl AsRef<str>) -> Result<String, String> {
    let certs = read_certificates(cert_path.as_ref())?;
    let mut sns = Vec::new();

    for cert in certs {
        let parsed = parse_certificate(&cert)?;
        if parsed.signature_algorithm.contains("1.2.840.113549.1.1.") {
            sns.push(format!(
                "{:x}",
                md5::compute(format!("{}{}", parsed.issuer, parsed.serial_decimal))
            ));
        }
    }

    if sns.is_empty() {
        return Err("no RSA certificate found in Alipay root certificate file".to_string());
    }

    Ok(sns.join("_"))
}

/// Extracts a PEM public key from an X.509 certificate.
pub fn public_key_from_cert_path(cert_path: impl AsRef<str>) -> Result<String, String> {
    let cert = read_certificate(cert_path.as_ref())?;
    let parsed = parse_certificate(&cert)?;
    Ok(wrap_pem(
        &STANDARD.encode(parsed.public_key_der),
        "PUBLIC KEY",
    ))
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

#[derive(Debug)]
struct ParsedCertificate<'a> {
    serial_decimal: String,
    issuer: String,
    signature_algorithm: String,
    public_key_der: &'a [u8],
}

fn read_certificate(path: &str) -> Result<Vec<u8>, String> {
    read_certificates(path)?
        .into_iter()
        .next()
        .ok_or_else(|| format!("no certificate found in {path}"))
}

fn read_certificates(path: &str) -> Result<Vec<Vec<u8>>, String> {
    let data =
        fs::read(path).map_err(|error| format!("failed to read certificate {path}: {error}"))?;
    let text = String::from_utf8_lossy(&data);

    if text.contains("-----BEGIN CERTIFICATE-----") {
        let mut certs = Vec::new();
        let mut rest = text.as_ref();

        while let Some(begin) = rest.find("-----BEGIN CERTIFICATE-----") {
            rest = &rest[begin + "-----BEGIN CERTIFICATE-----".len()..];
            let Some(end) = rest.find("-----END CERTIFICATE-----") else {
                return Err(format!(
                    "invalid PEM certificate in {path}: missing END marker"
                ));
            };

            let body = rest[..end]
                .chars()
                .filter(|character| !character.is_ascii_whitespace())
                .collect::<String>();
            certs.push(
                STANDARD
                    .decode(body)
                    .map_err(|error| format!("failed to decode PEM certificate {path}: {error}"))?,
            );
            rest = &rest[end + "-----END CERTIFICATE-----".len()..];
        }

        return Ok(certs);
    }

    Ok(vec![data])
}

fn parse_certificate(der: &[u8]) -> Result<ParsedCertificate<'_>, String> {
    let (certificate, _) = der_read_element(der, 0x30)?;
    let (tbs, after_tbs) = der_read_element(certificate, 0x30)?;
    let (signature_algorithm, _) = der_read_element(after_tbs, 0x30)?;

    let mut cursor = tbs;
    if cursor.first() == Some(&0xa0) {
        let (_, rest) = der_read_any(cursor)?;
        cursor = rest;
    }

    let (serial, rest) = der_read_element(cursor, 0x02)?;
    let (tbs_signature_algorithm, rest) = der_read_element(rest, 0x30)?;
    let (issuer, rest) = der_read_element(rest, 0x30)?;
    let (_, rest) = der_read_element(rest, 0x30)?;
    let (_, rest) = der_read_element(rest, 0x30)?;
    let (subject_public_key_info, _) = der_read_any(rest)?;
    if subject_public_key_info.first() != Some(&0x30) {
        return Err("invalid certificate: missing subject public key info".to_string());
    }

    Ok(ParsedCertificate {
        serial_decimal: unsigned_integer_to_decimal(serial),
        issuer: parse_name(issuer)?,
        signature_algorithm: parse_algorithm_oid(signature_algorithm)
            .or_else(|| parse_algorithm_oid(tbs_signature_algorithm))
            .unwrap_or_default(),
        public_key_der: subject_public_key_info,
    })
}

fn parse_algorithm_oid(algorithm_identifier: &[u8]) -> Option<String> {
    let (oid, _) = der_read_element(algorithm_identifier, 0x06).ok()?;
    Some(oid_to_string(oid))
}

fn parse_name(name: &[u8]) -> Result<String, String> {
    let mut rest = name;
    let mut parts = Vec::new();

    while !rest.is_empty() {
        let (set, after_set) = der_read_element(rest, 0x31)?;
        let (sequence, _) = der_read_element(set, 0x30)?;
        let (oid, value_part) = der_read_element(sequence, 0x06)?;
        let (value, _) = der_read_any(value_part)?;

        parts.push(format!("{}={}", oid_name(oid), escape_dn_value(value)));
        rest = after_set;
    }

    parts.reverse();
    Ok(parts.join(","))
}

fn der_read_any(input: &[u8]) -> Result<(&[u8], &[u8]), String> {
    if input.len() < 2 {
        return Err("invalid DER element: truncated header".to_string());
    }

    let length_byte = input[1];
    let (length, header_len) = if length_byte & 0x80 == 0 {
        (length_byte as usize, 2)
    } else {
        let length_len = (length_byte & 0x7f) as usize;
        if length_len == 0 || length_len > 8 || input.len() < 2 + length_len {
            return Err("invalid DER element: bad long-form length".to_string());
        }

        let mut length = 0usize;
        for byte in &input[2..2 + length_len] {
            length = (length << 8) | (*byte as usize);
        }
        (length, 2 + length_len)
    };

    let end = header_len
        .checked_add(length)
        .ok_or_else(|| "invalid DER element: length overflow".to_string())?;
    if input.len() < end {
        return Err("invalid DER element: truncated body".to_string());
    }

    Ok((&input[..end], &input[end..]))
}

fn der_read_element(input: &[u8], expected_tag: u8) -> Result<(&[u8], &[u8]), String> {
    let (element, rest) = der_read_any(input)?;
    if element.first() != Some(&expected_tag) {
        return Err(format!(
            "invalid DER element: expected tag 0x{expected_tag:02x}, got 0x{:02x}",
            element.first().copied().unwrap_or_default()
        ));
    }

    let content_offset = der_content_offset(element)?;
    Ok((&element[content_offset..], rest))
}

fn der_content_offset(element: &[u8]) -> Result<usize, String> {
    if element.len() < 2 {
        return Err("invalid DER element: truncated header".to_string());
    }

    if element[1] & 0x80 == 0 {
        Ok(2)
    } else {
        Ok(2 + (element[1] & 0x7f) as usize)
    }
}

fn oid_name(oid: &[u8]) -> &'static str {
    match oid_to_string(oid).as_str() {
        "2.5.4.3" => "CN",
        "2.5.4.6" => "C",
        "2.5.4.7" => "L",
        "2.5.4.8" => "ST",
        "2.5.4.10" => "O",
        "2.5.4.11" => "OU",
        "1.2.840.113549.1.9.1" => "EMAILADDRESS",
        _ => "OID",
    }
}

fn oid_to_string(oid: &[u8]) -> String {
    if oid.is_empty() {
        return String::new();
    }

    let first = oid[0];
    let mut parts = vec![(first / 40).to_string(), (first % 40).to_string()];
    let mut value = 0u64;

    for byte in &oid[1..] {
        value = (value << 7) | u64::from(byte & 0x7f);
        if byte & 0x80 == 0 {
            parts.push(value.to_string());
            value = 0;
        }
    }

    parts.join(".")
}

fn escape_dn_value(value_der: &[u8]) -> String {
    let content = match der_content_offset(value_der) {
        Ok(offset) if offset <= value_der.len() => &value_der[offset..],
        _ => value_der,
    };
    let raw = String::from_utf8_lossy(content);
    let mut escaped = String::with_capacity(raw.len());

    for character in raw.chars() {
        match character {
            ',' | '+' | '"' | '\\' | '<' | '>' | ';' => {
                escaped.push('\\');
                escaped.push(character);
            }
            _ => escaped.push(character),
        }
    }

    escaped
}

fn unsigned_integer_to_decimal(bytes: &[u8]) -> String {
    let bytes = bytes
        .iter()
        .skip_while(|byte| **byte == 0)
        .copied()
        .collect::<Vec<_>>();
    if bytes.is_empty() {
        return "0".to_string();
    }

    let mut decimal = vec![0u8];
    for byte in bytes {
        let mut carry = byte as u16;
        for digit in decimal.iter_mut().rev() {
            let value = (*digit as u16) * 256 + carry;
            *digit = (value % 10) as u8;
            carry = value / 10;
        }
        while carry > 0 {
            decimal.insert(0, (carry % 10) as u8);
            carry /= 10;
        }
    }

    decimal
        .into_iter()
        .map(|digit| char::from(b'0' + digit))
        .collect()
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
