use crate::{
    application::next_gateway_application::ApplicationContext,
    filter::gateway_filter::GatewayFilter, route::route_service_manager::UpStream,
};

#[derive(Debug, Clone)]
pub struct SecureHeadersFilter;

impl GatewayFilter for SecureHeadersFilter {
    fn filter(
        &self,
        _ctx: &mut ApplicationContext,
        upstream: &mut UpStream,
    ) -> pingora::Result<()> {
        let response_header = match upstream.response_header.as_mut() {
            Some(response_header) => response_header,
            None => return Ok(()),
        };

        // Add the standard hardening headers without overriding explicit route responses.
        insert_if_missing(response_header, "X-Content-Type-Options", "nosniff");
        insert_if_missing(response_header, "X-Frame-Options", "DENY");
        insert_if_missing(response_header, "Referrer-Policy", "no-referrer");
        insert_if_missing(response_header, "X-XSS-Protection", "1; mode=block");
        insert_if_missing(
            response_header,
            "Strict-Transport-Security",
            "max-age=31536000; includeSubDomains",
        );
        insert_if_missing(
            response_header,
            "Permissions-Policy",
            "geolocation=(), microphone=(), camera=()",
        );

        Ok(())
    }
}

fn insert_if_missing(response_header: &mut pingora::http::ResponseHeader, name: &str, value: &str) {
    if !response_header.headers.contains_key(name) {
        response_header.insert_header(name.to_string(), value).ok();
    }
}
