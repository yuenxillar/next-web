use std::collections::BTreeMap;

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

pub trait SignerV3<E = String> {
    fn sign(
        &self,
        method: &str,
        uri: &str,
        query_params: Option<&BTreeMap<&str, String>>,
        headers: &BTreeMap<&str, String>,
        body: impl AsRef<[u8]>,
        secret_key: &str,
        secret_key_id: &str,
        algorithm: &str,
    ) -> Result<String, E>;

    fn canonical_request<'a>(
        &self,
        method: &str,
        uri: &str,
        query_params: Option<&BTreeMap<&'a str, String>>,
        headers: &BTreeMap<&'a str, String>,
        body_sha256: &str,
    ) -> Result<String, E>;

    fn string_to_sign(
        &self,
        canonical_request: &str,
        algorithm: &str,
        headers: &BTreeMap<&str, String>,
    ) -> Result<String, E>;

    fn signature(
        &self,
        string_to_sign: &str,
        secret_key: &str,
        headers: &BTreeMap<&str, String>,
    ) -> Result<String, E>;

    fn hmac_sha256(&self, key: &[u8], message: &[u8]) -> Result<Vec<u8>, String> {
        let mut mac = Hmac::<Sha256>::new_from_slice(key)
            .map_err(|e| format!("use data key on sha256 fail:{}", e))?;
        mac.update(message);
        Ok(mac.finalize().into_bytes().to_vec())
    }

    fn hex_sha256(&self, message: impl AsRef<[u8]>) -> String {
        hex::encode(Sha256::digest(message))
    }

    fn sored_encoded_query_string(&self, params: Option<&BTreeMap<&str, String>>) -> String {
        params
            .map(|param| {
                param
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
                    .collect::<Vec<_>>()
                    .join("&")
            })
            .unwrap_or_default()
    }
}
