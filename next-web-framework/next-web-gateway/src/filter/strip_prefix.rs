use crate::application::next_gateway_application::ApplicationContext;

use super::gateway_filter::DefaultGatewayFilter;

#[derive(Debug, Clone)]
pub struct StripPrefixFilter {
    pub offset: usize,
}

impl DefaultGatewayFilter for StripPrefixFilter {
    fn filter(
        &self,
        _ctx: &mut ApplicationContext,
        request_header: &mut pingora::http::RequestHeader,
        _respnose_header: &mut pingora::http::ResponseHeader,
    ) {
        let raw_path = String::from_utf8_lossy(request_header.raw_path());

        let mut path = String::new();
        raw_path
            .split("/")
            .into_iter()
            .enumerate()
            .filter(|(i, _)| i >= &self.offset)
            .for_each(|(_, s)| {
                path.push('/');
                path.push_str(s);
            });

        if !path.is_empty() {
            request_header.set_uri(path.parse().unwrap());
        }
    }
}
