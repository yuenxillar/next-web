use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

type HmacSha256 = Hmac<Sha256>;

pub struct TencentSignerV3<'a> {
    secret_id: &'a str,
    secret_key: &'a str,
    service: &'a str,
    region: &'a str,
}

impl<'a> TencentSignerV3<'a> {
    pub fn new(secret_id: &'a str, secret_key: &'a str, service: &'a str, region: &'a str) -> Self {
        Self {
            secret_id,
            secret_key,
            service,
            region,
        }
    }

    pub fn sign(
        &self,
        method: &str,
        path: &str,
        endpoint: &str,
        params: &BTreeMap<&'a str, String>,
        headers: &BTreeMap<&'a str, String>,
        body: &[u8],
    ) -> String {
        // 1. 获取当前时间
        let now: DateTime<Utc> = Utc::now();
        let timestamp = now.timestamp();
        let date = now.format("%Y-%m-%d").to_string();

        // 2. 计算Content-Sha256
        // 正文做 SHA256 哈希，然后十六进制编码，最后编码串转换成小写字母
        let content_sha256 = hex::encode(Sha256::digest(body));

        // 3. 构造规范请求
        let canonical_request =
            self.build_canonical_request(method, path, params, headers, &content_sha256, endpoint);

        println!("{}", canonical_request);
        // 4. 构造待签字符串
        let string_to_sign = self.build_string_to_sign(&date, timestamp, &canonical_request);

        println!("{}", string_to_sign);
        // 5. 计算签名
        let signature = self.calculate_signature(&date, &string_to_sign);
        println!("{}", signature);

        // 6. 构造Authorization头
        self.build_authorization_header(&date, timestamp, &signature)
    }

    pub fn build_canonical_request(
        &self,
        method: &str,
        canonical_uri: &str,
        params: &BTreeMap<&str, String>,
        headers: &BTreeMap<&str, String>,
        payload: &str,
        endpoint: &str,
    ) -> String {
        // 规范化方法
        let canonical_method = method.to_uppercase();

        let hashed_request_payload = if canonical_method.eq("POST") {
            payload
        } else {
            ""
        };
        // 规范化查询字符串
        let canonical_query = if canonical_method.eq("POST") {
            String::from("")
        } else {
            params
                .iter()
                .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
                .collect::<Vec<_>>()
                .join("&")
        };
        // 规范化头
        let mut canonical_headers = headers
            .iter()
            .map(|(k, v)| {
                let key = k.trim().to_lowercase();
                let value = v.trim().to_lowercase();
                (key.to_string(), value.to_string())
            })
            .collect::<BTreeMap<_, _>>();

        canonical_headers.insert("content-type".into(), "application/json;".into());
        canonical_headers.insert("host".into(), endpoint.into());

        let signed_headers = canonical_headers
            .keys()
            .map(|k| k.as_str())
            .collect::<Vec<_>>()
            .join(";");

        let canonical_headers = canonical_headers
            .iter()
            .map(|(k, v)| format!("{}:{}", k, v))
            .collect::<Vec<_>>()
            .join("\n");

        // 组合规范请求
        format!(
            "{}\n{}\n{}\n{}\n\n{}\n{}",
            canonical_method,
            canonical_uri,
            canonical_query,
            canonical_headers,
            signed_headers,
            hashed_request_payload
        )
    }

    fn build_string_to_sign(&self, date: &str, timestamp: i64, canonical_request: &str) -> String {
        let credential_scope = format!("{}/{}/tc3_request", date, self.service);

        let hashed_canonical_request = hex::encode(Sha256::digest(canonical_request.as_bytes()));

        format!(
            "TC3-HMAC-SHA256\n{}\n{}\n{}",
            timestamp, credential_scope, hashed_canonical_request
        )
    }

    fn calculate_signature(&self, date: &str, string_to_sign: &str) -> String {
        // 计算派生签名密钥
        let secret = format!("TC3{}", self.secret_key);
        let secret_date = hmac_sha256(date.as_bytes(), secret.as_bytes());
        let secret_service = hmac_sha256(self.service.as_bytes(), &secret_date);
        let secret_signing = hmac_sha256("tc3_request".as_bytes(), &secret_service);

        // 计算签名
        hex::encode(hmac_sha256(string_to_sign.as_bytes(), &secret_signing))
    }

    fn build_authorization_header(&self, date: &str, timestamp: i64, signature: &str) -> String {
        let credential_scope = format!("{}/{}/{}/tc3_request", date, self.service, self.region);

        format!(
            "TC3-HMAC-SHA256 Credential={}/{}, SignedHeaders=content-sha256;host, Signature={}",
            self.secret_id, credential_scope, signature
        )
    }
}

fn hmac_sha256(message: &[u8], key: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).unwrap();
    mac.update(message);
    mac.finalize().into_bytes().to_vec()
}
