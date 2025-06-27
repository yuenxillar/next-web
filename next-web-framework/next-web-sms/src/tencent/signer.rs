use std::collections::BTreeMap;

use reqwest::Method;

use crate::core::signer::SignerV3;

const EMPTY_BODY_HEX_HASH_256: &'static str =
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

#[derive(Debug)]
pub struct TencentCloudSigner<'a> {
    pub(crate) service: &'a str,
}

impl <'a> TencentCloudSigner<'a> {
    
    pub fn new(service: &'a str) -> Self {
        Self { service }
    }
}

impl<'a> SignerV3 for TencentCloudSigner<'a> {
    fn sign(
        &self,
        method: &str,
        path: &str,
        query_params: Option<&BTreeMap<&str, String>>,
        headers: &BTreeMap<&str, String>,
        body: impl AsRef<[u8]>,
        secret_key: &str,
        secret_key_id: &str,
        algorithm: &str,
    ) -> Result<String, String> {
        let body = body.as_ref();
        let body_sha256 = if method.to_uppercase().eq(Method::GET.as_str()) {
            EMPTY_BODY_HEX_HASH_256.to_string()
        } else {
            self.hex_sha256(body)
        };

        let canonical_request =
            self.canonical_request(method, path, query_params, headers, &body_sha256)?;

        let string_to_sign = self.string_to_sign(&canonical_request, algorithm, headers)?;

        let signed_headers = headers
            .iter()
            .map(|(k, _)| k.to_lowercase())
            .collect::<Vec<_>>()
            .join(";");

        let signature = self.signature(&string_to_sign, secret_key, headers)?;

        let credential_scope = format!(
            "{}/{}/tc3_request",
            chrono::Utc::now().format("%Y-%m-%d"),
            self.service
        );
        Ok(format!(
            "{} Credential={}/{},SignedHeaders={},Signature={}",
            algorithm, secret_key_id, credential_scope, signed_headers, signature
        ))
    }

    fn canonical_request<'b>(
        &self,
        method: &str,
        path: &str,
        query_params: Option<&BTreeMap<&'b str, String>>,
        headers: &BTreeMap<&'b str, String>,
        body_sha256: &str,
    ) -> Result<String, String> {
        let method = method.to_uppercase();

        let canonical_query = self.sored_encoded_query_string(query_params);

        let _headers = headers
            .iter()
            .map(|(k, v)| {
                let key = k.trim().to_lowercase();
                let value = v.trim().to_lowercase();
                (key, value)
            })
            .collect::<BTreeMap<_, _>>();

        // content-type 和 host 为必选头部。
        let signed_headers = _headers
            .keys()
            .map(|k| k.as_str())
            .collect::<Vec<_>>()
            .join(";");

        let canonical_headers = _headers
            .iter()
            .map(|(k, v)| format!("{}:{}", k, v))
            .collect::<Vec<_>>()
            .join("\n");

        Ok(format!(
            "{}\n{}\n{}\n{}\n\n{}\n{}",
            method, path, canonical_query, canonical_headers, signed_headers, body_sha256
        ))
    }

    fn string_to_sign(
        &self,
        canonical_request: &str,
        algorithm: &str,
        headers: &BTreeMap<&str, String>,
    ) -> Result<String, String> {
        let hashed_canonical_request = self.hex_sha256(canonical_request);
        let timestamp_str = headers.get("X-TC-Timestamp").unwrap();
        let timestamp: i64 = timestamp_str
            .parse()
            .map_err(|e: std::num::ParseIntError| {
                format!("timestamp parse error!{}", e.to_string())
            })?;

        let datetime = chrono::Utc::now().format("%Y-%m-%d");

        let credential_scope = format!("{}/{}/tc3_request", datetime, self.service);
        Ok(format!(
            "{}\n{}\n{}\n{}",
            algorithm, timestamp, credential_scope, hashed_canonical_request
        ))
    }

    fn signature(
        &self,
        string_to_sign: &str,
        secret_key: &str,
        _headers: &BTreeMap<&str, String>,
    ) -> Result<String, String> {
        let date = chrono::Utc::now().format("%Y-%m-%d").to_string();

        // SecretDate = HMAC_SHA256("TC3" + SecretKey, Date)
        let secret_date =
            self.hmac_sha256(format!("TC3{}", secret_key).as_bytes(), date.as_bytes())?;
        // SecretService = HMAC_SHA256(SecretDate, Service)
        let secret_service = self.hmac_sha256(&secret_date, self.service.as_bytes())?;

        // SecretSigning = HMAC_SHA256(SecretService, "tc3_request")
        let secret_signing = self.hmac_sha256(&secret_service, "tc3_request".as_bytes())?;

        let signature = self.hmac_sha256(&secret_signing, string_to_sign.as_bytes())?;
        let data_sign = hex::encode(&signature);
        Ok(data_sign)
    }
}
