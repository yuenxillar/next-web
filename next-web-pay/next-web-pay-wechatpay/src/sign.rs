use std::collections::BTreeMap;

use rand::distributions::{Alphanumeric, DistString};

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
