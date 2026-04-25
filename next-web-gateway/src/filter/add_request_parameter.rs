use crate::route::route_service_manager::UpStream;
use crate::{filter::gateway_filter::GatewayFilter, util::key_value::KeyValue};
use form_urlencoded::{parse, Serializer};
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
        upstream: &mut UpStream,
    ) -> pingora::Result<()> {
        let request_header = match upstream.request_header.as_mut() {
            Some(request_header) => request_header,
            None => return Ok(()),
        };

        // Skip the rewrite when there is nothing to merge into the query string.
        if self.parameters.is_empty() {
            return Ok(());
        }

        let uri = &request_header.uri;
        let path = uri.path();
        let query = uri.query().unwrap_or("");
        let scheme = uri.scheme().map(|s| s.as_str()).unwrap_or("");
        let authority = uri.authority().map(|a| a.as_str()).unwrap_or("");

        // Merge the configured parameters on top of the existing query string.
        let new_query = merge_queries(query, &self.parameters);

        // Rebuild the URI using the original path and authority.
        let new_uri_str = build_uri_string(scheme, authority, path, &new_query);

        // Parse and update the proxied request URI in place.
        match new_uri_str.parse() {
            Ok(new_uri) => request_header.set_uri(new_uri),
            Err(error) => {
                warn!(
                    target: "gateway_filter",
                    "Failed to parse modified URI: {}, original_uri: {}",
                    error,
                    uri
                );
            }
        }

        Ok(())
    }
}

/// Merge query parameters where new values override existing keys.
fn merge_queries(original_query: &str, new_parameters: &[KeyValue<String, String>]) -> String {
    if new_parameters.is_empty() {
        return original_query.to_string();
    }

    // A map is enough here because later parameters are meant to override earlier ones.
    let mut param_map: HashMap<String, String> = HashMap::new();

    // Load the original query first.
    for (key, value) in parse(original_query.as_bytes()) {
        param_map.insert(key.into_owned(), value.into_owned());
    }

    // Then overwrite with the configured parameters.
    for param in new_parameters {
        param_map.insert(param.k.clone(), param.v.clone());
    }

    // Serialize the merged map back into a query string.
    let mut serializer = Serializer::new(String::with_capacity(
        original_query.len() + new_parameters.len() * 20,
    ));
    for (key, value) in param_map {
        serializer.append_pair(&key, &value);
    }

    serializer.finish()
}

/// Build a full URI string from the individual URI components.
fn build_uri_string(scheme: &str, authority: &str, path: &str, query: &str) -> String {
    let mut uri_string = String::new();

    // Preserve the original scheme when the upstream request uses an absolute URI.
    if !scheme.is_empty() {
        uri_string.push_str(scheme);
        uri_string.push_str("://");
    }

    // Preserve the authority for absolute URIs as well.
    if !authority.is_empty() {
        uri_string.push_str(authority);
    }

    // The path is always required.
    uri_string.push_str(path);

    // Append the query only when it exists.
    if !query.is_empty() {
        uri_string.push('?');
        uri_string.push_str(query);
    }

    uri_string
}
