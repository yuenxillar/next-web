//! `AddRequestParameter` filter: appends query parameters to the upstream request.

use async_trait::async_trait;
use form_urlencoded::Serializer;

use crate::error::GatewayError;
use crate::filter::gateway_filter::GatewayFilter;
use crate::filter::gateway_filter_chain::GatewayFilterChain;
use crate::server::ServerWebExchange;
use crate::util::key_value::KeyValue;

/// Appends configured query parameters to the upstream request.
#[derive(Debug, Clone)]
pub struct AddRequestParameterFilter {
    /// Parameters appended in declaration order.
    pub parameters: Vec<KeyValue<String, String>>,
}

#[async_trait]
impl GatewayFilter for AddRequestParameterFilter {
    async fn filter(
        &self,
        exchange: &mut dyn ServerWebExchange,
        chain: &dyn GatewayFilterChain,
    ) -> Result<(), GatewayError> {
        let request_header = match exchange.request_header() {
            Some(request_header) => request_header,
            None => return chain.filter(exchange).await,
        };

        if self.parameters.is_empty() {
            return chain.filter(exchange).await;
        }

        let uri = request_header.uri.clone();
        let query = merge_query(uri.query().unwrap_or(""), &self.parameters);
        let path_and_query = format!("{}?{query}", uri.path());

        let rebuilt = match uri.authority() {
            Some(authority) => match uri.scheme_str() {
                Some(scheme) => format!("{scheme}://{authority}{path_and_query}"),
                None => path_and_query,
            },
            None => path_and_query,
        };

        match rebuilt.parse() {
            Ok(uri) => request_header.set_uri(uri),
            Err(_) => {
                tracing::warn!(
                    "failed to rebuild the request URI after adding parameters: {rebuilt}"
                );
            }
        }

        chain.filter(exchange).await
    }
}

/// Append parameters to the query string without touching existing pairs.
///
/// The original query is kept verbatim: re-encoding it would turn `%20` into
/// `+`, reorder keys and collapse duplicates.
pub(crate) fn merge_query(original_query: &str, parameters: &[KeyValue<String, String>]) -> String {
    let appended = parameters
        .iter()
        .map(|parameter| {
            let mut pair = Serializer::new(String::new());
            pair.append_pair(&parameter.k, &parameter.v);
            pair.finish()
        })
        .collect::<Vec<_>>()
        .join("&");

    match (original_query.is_empty(), appended.is_empty()) {
        (true, _) => appended,
        (false, true) => original_query.to_owned(),
        (false, false) => format!("{original_query}&{appended}"),
    }
}

#[cfg(test)]
mod tests {
    use super::merge_query;
    use crate::util::key_value::KeyValue;

    fn parameters() -> Vec<KeyValue<String, String>> {
        vec![KeyValue::from(("s", "1"))]
    }

    #[test]
    fn add_request_parameter_preserves_duplicate_keys_and_order() {
        assert_eq!(merge_query("t=a&t=b", &parameters()), "t=a&t=b&s=1");
    }

    #[test]
    fn add_request_parameter_appends_to_an_empty_query() {
        assert_eq!(merge_query("", &parameters()), "s=1");
    }

    #[test]
    fn add_request_parameter_keeps_percent_encoding() {
        assert_eq!(merge_query("q=a%20b", &parameters()), "q=a%20b&s=1");
    }
}
