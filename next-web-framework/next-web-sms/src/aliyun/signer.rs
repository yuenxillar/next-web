use std::collections::BTreeMap;

use crate::core::signer::SignerV3;

pub struct AliyunSigner<'a> {
    pub(crate) map: &'a str,
}

impl<'a> SignerV3 for AliyunSigner<'a> {
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
    ) -> Result<String, String> {
        let body = body.as_ref();
        let body_sha256 = self.hex_sha256(body);
        let canonical_request =
            self.canonical_request(method, uri, query_params, headers, &body_sha256)?;
        let string_to_sign = self.string_to_sign(&canonical_request, algorithm)?;
        let signed_headers = headers
            .iter()
            .map(|(k, _)| k.to_string())
            .collect::<Vec<_>>()
            .join(";");

        let signature = self.signature(&string_to_sign, secret_key)?;
        Ok(format!(
            "{} Credential={},SignedHeaders={},Signature={}",
            algorithm, secret_key_id, signed_headers, signature
        ))
    }

    fn canonical_request<'b>(
        &self,
        method: &str,
        uri: &str,
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
                (key.to_string(), v.to_string())
            })
            .collect::<BTreeMap<_, _>>();

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
            method, uri, canonical_query, canonical_headers, signed_headers, body_sha256
        ))
    }

    fn string_to_sign(&self, canonical_request: &str, algorithm: &str) -> Result<String, String> {
        let hashed_canonical_request = self.hex_sha256(canonical_request);
        Ok(format!("{}\n{}", algorithm, hashed_canonical_request))
    }

    fn signature(&self, string_to_sign: &str, secret_key: &str) -> Result<String, String> {
        let signature = self.hmac_sha256(string_to_sign.as_bytes(), secret_key.as_bytes())?;
        let data_sign = hex::encode(&signature);
        Ok(data_sign)
    }
}
