use crate::{filter::gateway_filter::GatewayFilter, util::key_value::KeyValue};
use form_urlencoded::{parse, Serializer};
use pingora::http::ResponseHeader;
use pingora::prelude::RequestHeader;
use std::collections::HashMap;
use tracing::warn;

#[derive(Debug, Clone)]
pub struct AddRequestParameterFilter {
    pub parameters: Vec<KeyValue<String, String>>,
}

impl GatewayFilter for AddRequestParameterFilter {
    fn filter(
        &self,
        _ctx: &mut crate::application::next_gateway_application::ApplicationContext,
        request_header: &mut RequestHeader,
        _response_header: &mut ResponseHeader,
    ) {
        // 如果没有参数需要添加，直接返回
        if self.parameters.is_empty() {
            return;
        }

        let uri = &request_header.uri;
        let path = uri.path();
        let query = uri.query().unwrap_or("");
        let scheme = uri.scheme().map(|s| s.as_str()).unwrap_or("");
        let authority = uri.authority().map(|a| a.as_str()).unwrap_or("");

        // 构建新的查询字符串（新参数覆盖旧参数）
        let new_query = merge_queries(query, &self.parameters);

        // 构建新的 URI 字符串
        let new_uri_str = build_uri_string(scheme, authority, path, &new_query);

        // 解析并设置新 URI
        match new_uri_str.parse() {
            Ok(new_uri) => {
                request_header.set_uri(new_uri);
            }
            Err(e) => {
                warn!(
                    target: "gateway_filter",
                    "Failed to parse modified URI: {}, original_uri: {}",
                    e,
                    uri
                );
            }
        }
    }
}

/// 合并查询参数（新参数覆盖旧参数）
fn merge_queries(original_query: &str, new_parameters: &[KeyValue<String, String>]) -> String {
    if new_parameters.is_empty() {
        return original_query.to_string();
    }

    // 使用 HashMap 来高效处理参数覆盖
    let mut param_map: HashMap<String, String> = HashMap::new();

    // 1. 先解析原始查询参数
    for (k, v) in parse(original_query.as_bytes()) {
        param_map.insert(k.into_owned(), v.into_owned());
    }

    // 2. 用新参数覆盖旧参数
    for param in new_parameters {
        param_map.insert(param.k.clone(), param.v.clone());
    }

    // 3. 重新构建查询字符串
    let mut serializer = Serializer::new(String::with_capacity(
        original_query.len() + new_parameters.len() * 20,
    ));
    for (key, value) in param_map {
        serializer.append_pair(&key, &value);
    }

    serializer.finish()
}

/// 构建完整的 URI 字符串
fn build_uri_string(scheme: &str, authority: &str, path: &str, query: &str) -> String {
    let mut uri_string = String::new();

    // 添加 scheme
    if !scheme.is_empty() {
        uri_string.push_str(scheme);
        uri_string.push_str("://");
    }

    // 添加 authority
    if !authority.is_empty() {
        uri_string.push_str(authority);
    }

    // 添加 path
    uri_string.push_str(path);

    // 添加 query（如果有）
    if !query.is_empty() {
        uri_string.push('?');
        uri_string.push_str(query);
    }

    uri_string
}
