use async_trait::async_trait;

use crate::error::GatewayError;
use crate::filter::gateway_filter::GatewayFilter;
use crate::filter::gateway_filter_chain::GatewayFilterChain;
use crate::server::ServerWebExchange;
use std::collections::HashSet;

use bytes::Bytes;
use serde_json::Value;
use tracing::warn;

#[derive(Debug, Clone, Default)]
pub struct RemoveJsonAttributesResponseBodyFilter {
    pub names: Vec<String>,
    pub recursive: bool,
}

impl From<&str> for RemoveJsonAttributesResponseBodyFilter {
    fn from(value: &str) -> Self {
        let mut parts: Vec<&str> = value
            .split(',')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect();

        let recursive = if parts.len() > 1 {
            parts.last().and_then(|part| part.parse::<bool>().ok())
        } else {
            None
        };

        if recursive.is_some() {
            parts.pop();
        }

        Self {
            names: parts.into_iter().map(String::from).collect(),
            recursive: recursive.unwrap_or(false),
        }
    }
}

#[async_trait]
impl GatewayFilter for RemoveJsonAttributesResponseBodyFilter {
    async fn filter(
        &self,
        exchange: &mut dyn ServerWebExchange,
        chain: &dyn GatewayFilterChain,
    ) -> Result<(), GatewayError> {
        if self.names.is_empty() {
            return chain.filter(exchange).await;
        }

        let response_body = match exchange.response_body() {
            Some(response_body) => response_body,
            None => return chain.filter(exchange).await,
        };

        if response_body.is_empty() {
            return chain.filter(exchange).await;
        }

        let names: HashSet<&str> = self.names.iter().map(String::as_str).collect();
        let mut json = match serde_json::from_slice::<Value>(response_body.as_ref()) {
            Ok(json) => json,
            Err(error) => {
                warn!(
                    target: "gateway_filter",
                    "Failed to parse response body as JSON for RemoveJsonAttributesResponseBody: {}",
                    error
                );
                return chain.filter(exchange).await;
            }
        };

        let removed_any = if self.recursive {
            remove_attributes_recursively(&mut json, &names)
        } else {
            remove_attributes_from_root(&mut json, &names)
        };

        if !removed_any {
            return chain.filter(exchange).await;
        }

        match serde_json::to_vec(&json) {
            Ok(serialized) => *response_body = Bytes::from(serialized),
            Err(error) => warn!(
                target: "gateway_filter",
                "Failed to serialize JSON after RemoveJsonAttributesResponseBody: {}",
                error
            ),
        }

        chain.filter(exchange).await
    }
}

fn remove_attributes_from_root(value: &mut Value, names: &HashSet<&str>) -> bool {
    let Some(object) = value.as_object_mut() else {
        return false;
    };

    let original_len = object.len();
    object.retain(|key, _| !names.contains(key.as_str()));
    object.len() != original_len
}

fn remove_attributes_recursively(value: &mut Value, names: &HashSet<&str>) -> bool {
    match value {
        Value::Object(object) => {
            let original_len = object.len();
            object.retain(|key, _| !names.contains(key.as_str()));
            let mut removed_any = object.len() != original_len;

            for child in object.values_mut() {
                removed_any |= remove_attributes_recursively(child, names);
            }

            removed_any
        }
        Value::Array(array) => {
            let mut removed_any = false;
            for item in array.iter_mut() {
                removed_any |= remove_attributes_recursively(item, names);
            }
            removed_any
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;

    use super::RemoveJsonAttributesResponseBodyFilter;
    use crate::filter::gateway_filter::GatewayFilter;

    fn test_ctx() -> crate::context::RequestContext {
        crate::context::RequestContext {
            fallback_id: None,
            route_id: None,
            original_request_path: None,
            buffer_response_body: false,
            response_body_buffer: Vec::new(),
            local_response_cache_manager: None,
            local_response_cache_request: None,
            pending_local_response_cache: None,
            session: None,
            direct_response: None,
        }
    }

    fn apply_filter(body: &str, config: &str) -> String {
        let filter = RemoveJsonAttributesResponseBodyFilter::from(config);
        let mut ctx = test_ctx();
        let mut response_body = Some(Bytes::from(body.to_string()));
        let mut exchange = crate::server::DefaultServerWebExchange::new(&mut ctx, crate::context::HeaderAndBody::with_resp_body(&mut response_body));

        futures::executor::block_on(filter.filter(&mut exchange, &crate::handler::DefaultGatewayFilterChain::default())).unwrap();
        drop(exchange);

        String::from_utf8(response_body.unwrap().to_vec()).unwrap()
    }

    #[test]
    fn parses_optional_recursive_flag() {
        let filter = RemoveJsonAttributesResponseBodyFilter::from("id,color,true");

        assert_eq!(filter.names, vec!["id".to_string(), "color".to_string()]);
        assert!(filter.recursive);
    }

    #[test]
    fn removes_attributes_only_from_root_by_default() {
        let body = r#"{"id":1,"color":"red","nested":{"id":2,"name":"child"}}"#;

        let filtered = apply_filter(body, "id,color");

        assert_eq!(filtered, r#"{"nested":{"id":2,"name":"child"}}"#);
    }

    #[test]
    fn removes_attributes_recursively_from_nested_objects_and_arrays() {
        let body = r#"{"id":1,"items":[{"id":2,"name":"a"},{"name":"b","child":{"id":3}}],"child":{"color":"red","keep":true}}"#;

        let filtered = apply_filter(body, "id,color,true");

        assert_eq!(
            filtered,
            r#"{"child":{"keep":true},"items":[{"name":"a"},{"child":{},"name":"b"}]}"#
        );
    }
}
