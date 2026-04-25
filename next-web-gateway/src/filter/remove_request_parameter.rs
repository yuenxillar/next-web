use std::collections::HashSet;

use form_urlencoded::{parse, Serializer};
use pingora::http::RequestHeader;
use tracing::warn;

use crate::{
    application::next_gateway_application::ApplicationContext,
    filter::gateway_filter::GatewayFilter, route::route_service_manager::UpStream,
};

#[derive(Debug, Clone)]
pub struct RemoveRequestParameterFilter {
    pub names: Vec<String>,
}

impl GatewayFilter for RemoveRequestParameterFilter {
    fn filter(
        &self,
        _ctx: &mut ApplicationContext,
        upstream: &mut UpStream,
    ) -> pingora::Result<()> {
        if self.names.is_empty() {
            return Ok(());
        }

        let request_header = match upstream.request_header.as_mut() {
            Some(request_header) => request_header,
            None => return Ok(()),
        };

        let query = match request_header.uri.query() {
            Some(query) if !query.is_empty() => query,
            _ => return Ok(()),
        };

        let names: HashSet<&str> = self.names.iter().map(String::as_str).collect();
        let mut removed_any = false;
        let mut serializer = Serializer::new(String::with_capacity(query.len()));

        for (name, value) in parse(query.as_bytes()) {
            if names.contains(name.as_ref()) {
                removed_any = true;
                continue;
            }

            serializer.append_pair(name.as_ref(), value.as_ref());
        }

        if !removed_any {
            return Ok(());
        }

        let new_query = serializer.finish();
        rewrite_request_uri(request_header, &new_query);

        Ok(())
    }
}

fn rewrite_request_uri(request_header: &mut RequestHeader, query: &str) {
    let uri = &request_header.uri;
    let path = uri.path();
    let scheme = uri.scheme().map(|value| value.as_str()).unwrap_or("");
    let authority = uri.authority().map(|value| value.as_str()).unwrap_or("");

    let new_uri = build_uri_string(scheme, authority, path, query);

    match new_uri.parse() {
        Ok(uri) => request_header.set_uri(uri),
        Err(error) => warn!(
            target: "gateway_filter",
            "Failed to parse modified URI: {}, original_uri: {}",
            error,
            request_header.uri
        ),
    }
}

fn build_uri_string(scheme: &str, authority: &str, path: &str, query: &str) -> String {
    let mut uri_string = String::new();

    if !scheme.is_empty() {
        uri_string.push_str(scheme);
        uri_string.push_str("://");
    }

    if !authority.is_empty() {
        uri_string.push_str(authority);
    }

    uri_string.push_str(path);

    if !query.is_empty() {
        uri_string.push('?');
        uri_string.push_str(query);
    }

    uri_string
}

#[cfg(test)]
mod tests {
    use pingora::http::RequestHeader;

    use crate::{
        application::next_gateway_application::ApplicationContext,
        route::route_service_manager::UpStream,
    };

    use super::RemoveRequestParameterFilter;
    use crate::filter::gateway_filter::GatewayFilter;

    fn test_ctx() -> ApplicationContext {
        ApplicationContext {
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

    fn apply_filter(raw_path: &[u8], names: &[&str]) -> RequestHeader {
        let mut request = RequestHeader::build("GET", raw_path, None).unwrap();
        let filter = RemoveRequestParameterFilter {
            names: names.iter().map(|name| name.to_string()).collect(),
        };
        let mut ctx = test_ctx();
        let mut upstream = UpStream::from_request_header(&mut request);

        filter.filter(&mut ctx, &mut upstream).unwrap();
        drop(upstream);

        request
    }

    #[test]
    fn removes_all_matching_parameters_and_drops_empty_query() {
        let request = apply_filter(b"/search?token=1&token=2", &["token"]);

        assert_eq!(request.uri.path(), "/search");
        assert_eq!(request.uri.query(), None);
        assert_eq!(
            request.uri.path_and_query().map(|value| value.as_str()),
            Some("/search")
        );
    }

    #[test]
    fn preserves_query_encoding_when_removing_other_parameters() {
        let request = apply_filter(
            b"/search?keep=1&encoded=hello%20world&plus=a%2Bb&drop=gone",
            &["drop"],
        );

        assert_eq!(
            request.uri.path_and_query().map(|value| value.as_str()),
            Some("/search?keep=1&encoded=hello+world&plus=a%2Bb")
        );
    }

    #[test]
    fn leaves_uri_unchanged_when_no_parameter_matches() {
        let request = apply_filter(b"/search?keep=1&encoded=hello%20world", &["drop"]);

        assert_eq!(
            request.uri.path_and_query().map(|value| value.as_str()),
            Some("/search?keep=1&encoded=hello%20world")
        );
    }
}
