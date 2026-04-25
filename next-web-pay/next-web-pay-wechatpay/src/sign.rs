use std::collections::BTreeMap;

use hmac::{Hmac, Mac};
use rand::distributions::{Alphanumeric, DistString};
use sha2::Sha256;

use crate::Result;
use crate::error::WechatPayError;
use crate::model::SignType;

type HmacSha256 = Hmac<Sha256>;

/// Creates a random nonce string.
pub fn generate_nonce_str() -> String {
    Alphanumeric.sample_string(&mut rand::thread_rng(), 32)
}

/// Builds the canonical signing string.
pub fn build_sign_content(params: &BTreeMap<String, String>, api_key: &str) -> String {
    let string_a = params
        .iter()
        .filter(|(key, value)| key.as_str() != "sign" && !value.is_empty())
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("&");

    if string_a.is_empty() {
        format!("key={api_key}")
    } else {
        format!("{string_a}&key={api_key}")
    }
}

/// Signs the parameter map.
pub fn sign_params(
    params: &BTreeMap<String, String>,
    api_key: &str,
    sign_type: SignType,
) -> Result<String> {
    let content = build_sign_content(params, api_key);
    let signature = match sign_type {
        SignType::Md5 => format!("{:x}", md5::compute(content)).to_uppercase(),
        SignType::HmacSha256 => {
            let mut mac = HmacSha256::new_from_slice(api_key.as_bytes())
                .map_err(|error| WechatPayError::Signing(error.to_string()))?;
            mac.update(content.as_bytes());
            hex::encode_upper(mac.finalize().into_bytes())
        }
    };

    Ok(signature)
}

/// Verifies the signature in a parameter map.
pub fn verify_params(
    params: &BTreeMap<String, String>,
    api_key: &str,
    sign_type: SignType,
) -> Result<bool> {
    let Some(signature) = params.get("sign") else {
        return Err(WechatPayError::MissingField("sign"));
    };
    let expected = sign_params(params, api_key, sign_type)?;
    Ok(expected == *signature)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::model::SignType;

    use super::{build_sign_content, sign_params, verify_params};

    #[test]
    fn build_sign_content_keeps_sorted_order() {
        let mut params = BTreeMap::new();
        params.insert("b".to_string(), "2".to_string());
        params.insert("a".to_string(), "1".to_string());
        params.insert("sign".to_string(), "ignored".to_string());

        let content = build_sign_content(&params, "secret");

        assert_eq!(content, "a=1&b=2&key=secret");
    }

    #[test]
    fn md5_sign_and_verify_work() {
        let mut params = BTreeMap::new();
        params.insert("appid".to_string(), "wx123".to_string());
        params.insert("mch_id".to_string(), "1900000109".to_string());
        params.insert("nonce_str".to_string(), "nonce".to_string());

        let sign = sign_params(&params, "secret", SignType::Md5).expect("sign");
        params.insert("sign".to_string(), sign);

        assert!(verify_params(&params, "secret", SignType::Md5).expect("verify"));
    }
}
