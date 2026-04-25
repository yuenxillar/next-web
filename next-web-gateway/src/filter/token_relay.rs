use crate::{
    application::next_gateway_application::ApplicationContext,
    route::route_service_manager::UpStream,
};

use super::gateway_filter::GatewayFilter;

#[derive(Debug, Clone, Default)]
pub struct TokenRelayFilter {}

impl GatewayFilter for TokenRelayFilter {
    fn filter(
        &self,
        _ctx: &mut ApplicationContext,
        upstream: &mut UpStream,
    ) -> pingora::Result<()> {
        let request_header = match upstream.request_header.as_mut() {
            Some(request_header) => request_header,
            None => return Ok(()),
        };

        let bearer_token = request_header
            .headers
            .get_all("authorization")
            .iter()
            .filter_map(|value| value.to_str().ok())
            .find(|value| value.len() > 7 && value[..7].eq_ignore_ascii_case("Bearer "))
            .map(str::to_string);

        let Some(bearer_token) = bearer_token else {
            return Ok(());
        };

        // Normalize the proxied Authorization header to a single Bearer token value.
        request_header
            .insert_header("Authorization", bearer_token)
            .ok();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use pingora::http::RequestHeader;

    use crate::{
        application::next_gateway_application::ApplicationContext,
        route::route_service_manager::UpStream,
    };

    use super::TokenRelayFilter;
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

    #[test]
    fn relays_incoming_bearer_token() {
        let mut request = RequestHeader::build("GET", b"/resource", None).unwrap();
        request
            .append_header("Authorization", "Bearer access-token")
            .unwrap();

        let filter = TokenRelayFilter::default();
        let mut ctx = test_ctx();
        let mut upstream = UpStream::from_request_header(&mut request);

        filter.filter(&mut ctx, &mut upstream).unwrap();
        drop(upstream);

        assert_eq!(
            request
                .headers
                .get("authorization")
                .and_then(|value| value.to_str().ok()),
            Some("Bearer access-token")
        );
    }

    #[test]
    fn ignores_non_bearer_authorization_headers() {
        let mut request = RequestHeader::build("GET", b"/resource", None).unwrap();
        request
            .insert_header("Authorization", "Basic ZGVtbzpwYXNz")
            .unwrap();

        let filter = TokenRelayFilter::default();
        let mut ctx = test_ctx();
        let mut upstream = UpStream::from_request_header(&mut request);

        filter.filter(&mut ctx, &mut upstream).unwrap();
        drop(upstream);

        assert_eq!(
            request
                .headers
                .get("authorization")
                .and_then(|value| value.to_str().ok()),
            Some("Basic ZGVtbzpwYXNz")
        );
    }
}
