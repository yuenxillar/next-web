use crate::{
    application::next_gateway_application::ApplicationContext,
    route::route_service_manager::UpStream,
};

use super::gateway_filter::GatewayFilter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DedupeStrategy {
    RetainFirst,
    RetainLast,
    RetainUnique,
}

impl Default for DedupeStrategy {
    fn default() -> Self {
        Self::RetainFirst
    }
}

impl DedupeStrategy {
    fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_uppercase().as_str() {
            "RETAIN_FIRST" => Some(Self::RetainFirst),
            "RETAIN_LAST" => Some(Self::RetainLast),
            "RETAIN_UNIQUE" => Some(Self::RetainUnique),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct DedupeResponseHeaderFilter {
    pub headers: Vec<String>,
    pub strategy: DedupeStrategy,
}

impl From<&str> for DedupeResponseHeaderFilter {
    fn from(value: &str) -> Self {
        let mut parts: Vec<String> = value
            .split(|c: char| c == ',' || c.is_whitespace())
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(String::from)
            .collect();

        let strategy = if let Some(strategy) = parts
            .last()
            .and_then(|part| DedupeStrategy::parse(part.as_str()))
        {
            parts.pop();
            strategy
        } else {
            DedupeStrategy::default()
        };

        Self {
            headers: parts,
            strategy,
        }
    }
}

impl GatewayFilter for DedupeResponseHeaderFilter {
    fn filter(
        &self,
        _ctx: &mut ApplicationContext,
        upstream: &mut UpStream,
    ) -> pingora::Result<()> {
        let response_header = match upstream.response_header.as_mut() {
            Some(response_header) => response_header,
            None => return Ok(()),
        };

        for header_name in self.headers.iter() {
            let existing_values: Vec<_> = response_header
                .headers
                .get_all(header_name.as_str())
                .iter()
                .cloned()
                .collect();

            if existing_values.len() <= 1 {
                continue;
            }

            let retained_values = match self.strategy {
                DedupeStrategy::RetainFirst => existing_values
                    .first()
                    .cloned()
                    .into_iter()
                    .collect::<Vec<_>>(),
                DedupeStrategy::RetainLast => existing_values
                    .last()
                    .cloned()
                    .into_iter()
                    .collect::<Vec<_>>(),
                DedupeStrategy::RetainUnique => {
                    let mut unique_values = Vec::new();
                    for value in existing_values {
                        if !unique_values.contains(&value) {
                            unique_values.push(value);
                        }
                    }
                    unique_values
                }
            };

            response_header.remove_header(header_name.as_str());

            for value in retained_values {
                response_header
                    .append_header(header_name.clone(), value)
                    .ok();
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use pingora::http::ResponseHeader;

    use crate::{
        application::next_gateway_application::ApplicationContext,
        route::route_service_manager::UpStream,
    };

    use super::{DedupeResponseHeaderFilter, DedupeStrategy};
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
    fn parses_space_delimited_headers_with_default_strategy() {
        let filter = DedupeResponseHeaderFilter::from(
            "Access-Control-Allow-Credentials Access-Control-Allow-Origin",
        );

        assert_eq!(
            filter.headers,
            vec![
                "Access-Control-Allow-Credentials".to_string(),
                "Access-Control-Allow-Origin".to_string()
            ]
        );
        assert_eq!(filter.strategy, DedupeStrategy::RetainFirst);
    }

    #[test]
    fn parses_optional_strategy_suffix() {
        let filter = DedupeResponseHeaderFilter::from(
            "Access-Control-Allow-Origin, Access-Control-Allow-Credentials, RETAIN_UNIQUE",
        );

        assert_eq!(
            filter.headers,
            vec![
                "Access-Control-Allow-Origin".to_string(),
                "Access-Control-Allow-Credentials".to_string()
            ]
        );
        assert_eq!(filter.strategy, DedupeStrategy::RetainUnique);
    }

    #[test]
    fn retains_first_value_by_default() {
        let mut response = ResponseHeader::build(200, Some(4)).unwrap();
        response
            .append_header("Access-Control-Allow-Origin", "https://a.example")
            .unwrap();
        response
            .append_header("Access-Control-Allow-Origin", "https://b.example")
            .unwrap();

        let filter = DedupeResponseHeaderFilter::from("Access-Control-Allow-Origin");
        let mut ctx = test_ctx();
        let mut upstream = UpStream::from_response_header(&mut response);

        filter.filter(&mut ctx, &mut upstream).unwrap();

        let values: Vec<_> = response
            .headers
            .get_all("Access-Control-Allow-Origin")
            .iter()
            .map(|value| value.to_str().unwrap().to_string())
            .collect();

        assert_eq!(values, vec!["https://a.example".to_string()]);
    }

    #[test]
    fn retains_all_unique_values_when_requested() {
        let mut response = ResponseHeader::build(200, Some(5)).unwrap();
        response
            .append_header("Access-Control-Allow-Origin", "https://a.example")
            .unwrap();
        response
            .append_header("Access-Control-Allow-Origin", "https://a.example")
            .unwrap();
        response
            .append_header("Access-Control-Allow-Origin", "https://b.example")
            .unwrap();

        let filter = DedupeResponseHeaderFilter::from("Access-Control-Allow-Origin RETAIN_UNIQUE");
        let mut ctx = test_ctx();
        let mut upstream = UpStream::from_response_header(&mut response);

        filter.filter(&mut ctx, &mut upstream).unwrap();

        let values: Vec<_> = response
            .headers
            .get_all("Access-Control-Allow-Origin")
            .iter()
            .map(|value| value.to_str().unwrap().to_string())
            .collect();

        assert_eq!(
            values,
            vec![
                "https://a.example".to_string(),
                "https://b.example".to_string()
            ]
        );
    }
}
