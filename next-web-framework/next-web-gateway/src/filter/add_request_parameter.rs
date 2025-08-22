use std::fmt::Write;

use crate::{filter::gateway_filter::GatewayFilter, util::key_value::KeyValue};
use tracing::warn;

#[derive(Debug, Clone)]
pub struct AddRequestParameterFilter {
    pub parameter: KeyValue<String>,
}

impl GatewayFilter for AddRequestParameterFilter {
    fn filter(
        &self,
        _ctx: &mut crate::application::next_gateway_application::ApplicationContext,
        request_header: &mut pingora::prelude::RequestHeader,
        _respnose_header: &mut pingora::http::ResponseHeader,
    ) {
        let key = &self.parameter.k;
        let value = &self.parameter.v;

        // 1. 获取当前 URI 的 path 和 query
        let uri = &request_header.uri;
        let path = uri.path();
        let query = uri.query();

        // 2. 构建新的 query string
        let mut new_query = String::new();

        // 原始 query 存在则先写入
        if let Some(q) = query {
            new_query.push_str(q);
        }

        // 添加分隔符
        if new_query.is_empty() {
            // 没有原始 query，直接拼接
            write!(
                &mut new_query,
                "{}={}",
                key,
                form_urlencoded::byte_serialize(value.as_bytes()).collect::<String>()
            )
            .unwrap();
        } else {
            // 已有 query，用 & 连接
            write!(
                &mut new_query,
                "&{}={}",
                key,
                form_urlencoded::byte_serialize(value.as_bytes()).collect::<String>()
            )
            .unwrap();
        }

        // 3. 重新构建 URI
        let mut new_uri = path.to_string();
        if !new_query.is_empty() {
            new_uri.push('?');
            new_uri.push_str(&new_query);
        }

        // 4. 更新 RequestHeader 的 URI
        match new_uri.parse() {
            Ok(parsed_uri) => {
                request_header.set_uri(parsed_uri);
            }
            Err(e) => {
                // 日志记录（如果有日志系统）
                warn!("Failed to parse modified URI '{}': {}", new_uri, e);
                // 可选择 panic、忽略或 fallback
            }
        }
    }
}
