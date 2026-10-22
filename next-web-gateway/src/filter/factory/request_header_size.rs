//! `RequestHeaderSize` filter: rejects requests whose header block is too large.

use async_trait::async_trait;

use crate::error::GatewayError;
use crate::filter::gateway_filter::GatewayFilter;
use crate::filter::gateway_filter_chain::GatewayFilterChain;
use crate::server::ServerWebExchange;

/// Rejects a request with 431 when its head is larger than `max_size` bytes.
#[derive(Debug, Clone)]
pub struct RequestHeaderSizeFilter {
    /// Maximum size of the request line plus headers, in bytes.
    pub max_size: u32,
    /// Body returned to the client when the limit is exceeded.
    pub error_message: String,
}

#[async_trait]
impl GatewayFilter for RequestHeaderSizeFilter {
    async fn filter(
        &self,
        exchange: &mut dyn ServerWebExchange,
        chain: &dyn GatewayFilterChain,
    ) -> Result<(), GatewayError> {
        let request_header = match exchange.request_header() {
            Some(request_header) => request_header,
            None => return chain.filter(exchange).await,
        };

        let size = head_size(
            request_header.method.as_str(),
            request_header
                .uri
                .path_and_query()
                .map(|value| value.as_str())
                .unwrap_or("/"),
            request_header
                .headers
                .iter()
                .map(|(name, value)| (name.as_str(), value.as_bytes())),
        );

        if size > self.max_size as usize {
            return exchange
                .request_context()
                .respond_with_text(431, Vec::new(), self.error_message.clone())
                .map_err(|_| GatewayError::ServerRejectsRequest);
        }

        chain.filter(exchange).await
    }
}

/// Size of the serialized request head: request line, headers and final CRLF.
pub(crate) fn head_size<'a>(
    method: &str,
    path_and_query: &str,
    headers: impl Iterator<Item = (&'a str, &'a [u8])>,
) -> usize {
    // "METHOD SP URI SP HTTP/1.1 CRLF"
    let request_line = method.len() + 1 + path_and_query.len() + 1 + "HTTP/1.1".len() + 2;

    let headers_size = headers
        .map(|(name, value)| name.len() + 2 + value.len() + 2)
        .sum::<usize>();

    request_line + headers_size + 2
}

#[cfg(test)]
mod tests {
    use super::head_size;

    #[test]
    fn request_header_size_counts_the_request_line() {
        let with_no_headers = head_size("GET", "/very/long/path", std::iter::empty());
        assert_eq!(
            with_no_headers,
            "GET /very/long/path HTTP/1.1\r\n".len() + 2
        );
    }

    #[test]
    fn request_header_size_counts_names_and_values() {
        let size = head_size(
            "GET",
            "/",
            [("host", b"example.com".as_slice())].into_iter(),
        );

        assert_eq!(size, "GET / HTTP/1.1\r\n".len() + "host: example.com\r\n".len() + 2);
    }
}
