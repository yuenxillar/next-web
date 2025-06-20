use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

type HmacSha256 = Hmac<Sha256>;


#[derive(Debug)]
pub struct SignerV3<'a> {
    secret_id: &'a str,
    secret_key: &'a str,
    service: &'a str,
    region: &'a str,
    algorithm: &'a str,
}

impl<'a> SignerV3<'a> {
    pub fn new(
        secret_id: &'a str,
        secret_key: &'a str,
        service: &'a str,
        region: &'a str,
        algorithm: &'a str,
    ) -> Self {
        Self {
            secret_id,
            secret_key,
            service,
            region,
            algorithm,
        }
    }

    #[tracing::instrument]
    pub fn sign(
        &self,
        method: &str,
        path: &str,
        params: &BTreeMap<&'a str, String>,
        headers: &BTreeMap<&'a str, String>,
        body: &[u8],
    ) -> Result<String, String> {
        // 1. 获取当前时间d
        let now: DateTime<Utc> = Utc::now();
        let timestamp = now.timestamp();
        let date = now.format("%Y-%m-%d").to_string();

        // 2. 计算Content-Sha256
        // 正文做 SHA256 哈希，然后十六进制编码，最后编码串转换成小写字母
        let content_sha256 = hex_sha256(body);

        // 3. 构造规范请求
        let canonical_request =
            self.build_canonical_request(method, path, params, headers, &content_sha256);
        println!("{}", canonical_request);

        // 4. 构造待签字符串
        let string_to_sign =
            self.build_string_to_sign(&canonical_request, self.algorithm);
        println!("{}", string_to_sign);

        // 5. 计算签名
        let signature = self.calculate_signature( &string_to_sign).unwrap();
        println!("{}", signature);

        // 6. 构造Authorization头
        let auth = self.build_authorization_header(&date, &signature);

        Ok(auth)
    }

    /// 构造规范化请求
    /// CanonicalRequest =
    /// HTTPRequestMethod + '\n' +      // http请求方法，全大写
    /// CanonicalURI + '\n' +           // 规范化URI
    /// CanonicalQueryString + '\n' +   // 规范化查询字符串
    /// CanonicalHeaders + '\n' +       // 规范化消息头
    /// SignedHeaders + '\n' +          // 已签名消息头
    /// HashedRequestPayload		    // 请求body的hash值
    pub fn build_canonical_request(
        &self,
        method: &str,
        canonical_uri: &str,
        params: &BTreeMap<&str, String>,
        headers: &BTreeMap<&str, String>,
        hashed_request_payload: &str,
    ) -> String {
        // 规范化方法
        let req_method = method.to_uppercase();

        // 规范化查询字符串
        let canonical_query = if req_method.eq("POST") {
            String::from("")
        } else {
            build_sored_encoded_query_string(params)
        };
        // 规范化头
        let _headers = headers
            .iter()
            .map(|(k, v)| {
                let key = k.trim().to_lowercase();
                let value = v.trim().to_lowercase();
                (key.to_string(), value.to_string())
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

        // 组合规范请求
        format!(
            "{}\n{}\n{}\n{}\n\n{}\n{}",
            req_method,
            canonical_uri,
            canonical_query,
            canonical_headers,
            signed_headers,
            hashed_request_payload
        )
    }

    fn build_string_to_sign(
        &self,
        // date: &str,
        // timestamp: i64,
        canonical_request: &str,
        algorithm: &str,
    ) -> String {
        // let credential_scope = format!("{}/{}/tc3_request", date, self.service);

        let hashed_canonical_request = hex_sha256(canonical_request);

        format!("{}\n{}", algorithm, hashed_canonical_request)

        // format!(
        //     "{}\n{}\n{}\n{}",
        //     algorithm, timestamp, credential_scope, hashed_canonical_request
        // )
    }

    // , date: &str,
    fn calculate_signature(&self, string_to_sign: &str) -> Result<String, String> {
        // 计算派生签名密钥
        // let secret = format!("TC3{}", self.secret_key);
        // let secret_date = hmac_sha256(date.as_bytes(), self.secret_key.as_bytes())?;
        // let secret_signing = hmac_sha256("tc3_request".as_bytes(), &secret_service)?;

        let signature = hmac_sha256(string_to_sign.as_bytes(), self.secret_key.as_bytes())?;

        // 计算签名
        // let data_sign = hex::encode(hmac_sha256(string_to_sign.as_bytes(), &secret_signing)?);
        let data_sign = hex::encode(&signature);

        Ok(data_sign)
    }

    fn build_authorization_header(&self, signed_geaders: &str, signature: &str) -> String {
        // let credential_scope = format!("{}/{}/{}/tc3_request", date, self.service, self.region);
        
        format!(
            "{} Credential={},SignedHeaders={},Signature={}",
            self.algorithm, self.secret_id, signed_geaders, signature
        )
    }
}

pub fn hmac_sha256(message: &[u8], key: &[u8]) -> Result<Vec<u8>, String> {
    let mut mac = HmacSha256::new_from_slice(key)
        .map_err(|e| format!("use data key on sha256 fail:{}", e))?;
    mac.update(message);
    Ok(mac.finalize().into_bytes().to_vec())
}

pub fn hex_sha256(message: impl AsRef<[u8]>) -> String {
    hex::encode(Sha256::digest(message))
}

pub(crate) fn build_sored_encoded_query_string(params: &BTreeMap<&str, String>) -> String {
    params
        .iter()
        .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("&")
}