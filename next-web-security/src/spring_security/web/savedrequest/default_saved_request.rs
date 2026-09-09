use std::{collections::HashMap, fmt};

use next_web_core::{
    http::{Cookie, HttpMethod},
    traits::http::http_request::HttpRequest,
    util::locale::Locale,
};

use crate::web::{savedrequest::SavedRequest, util::UrlUtils};

const HEADER_IF_NONE_MATCH: &str = "If-None-Match";
const HEADER_IF_MODIFIED_SINCE: &str = "If-Modified-Since";

/// Represents central information from a `HttpRequest`.
///
/// This class is used by `SavedRequestAwareWrapper` to reproduce the request after
/// successful authentication. An instance of this class is stored at the time of an
/// authentication exception by `ExceptionTranslationFilter`.
#[derive(Clone)]
pub struct DefaultSavedRequest {
    cookies: Vec<Cookie>,
    locales: Vec<Locale>,
    headers: HashMap<String, Vec<String>>,
    parameters: HashMap<String, Vec<String>>,
    context_path: Option<String>,
    method: String,
    path_info: Option<String>,
    query_string: Option<String>,
    request_uri: String,
    request_url: Option<String>,
    scheme: String,
    server_name: String,
    servlet_path: Option<String>,
    server_port: u16,
    matching_request_parameter_name: Option<String>,
}

impl DefaultSavedRequest {
    /// Creates a snapshot of the given request.
    ///
    /// # Arguments
    ///
    /// * `request` - the request to snapshot
    /// * `matching_request_parameter_name` - optional name of the request parameter
    ///   that allows matching a saved request to a later request
    pub fn from_request(
        request: &dyn HttpRequest,
        matching_request_parameter_name: Option<&str>,
    ) -> Self {
        let mut saved = Self {
            cookies: request.cookies().map(ToOwned::to_owned).unwrap_or_default(),
            locales: request.locales().unwrap_or_default(),
            headers: HashMap::new(),
            parameters: HashMap::new(),
            context_path: request.context_path().map(ToOwned::to_owned),
            method: request.method().to_string(),
            path_info: None,
            query_string: request.query().map(ToOwned::to_owned),
            request_uri: request.path().to_string(),
            request_url: Some(UrlUtils::build_full_request_url(request)),
            scheme: request.scheme().unwrap_or("http").to_string(),
            server_name: request.server_name().unwrap_or_default(),
            servlet_path: None,
            server_port: request.server_port().unwrap_or(80),
            matching_request_parameter_name: matching_request_parameter_name.map(ToOwned::to_owned),
        };

        for name in request.header_names() {
            // Skip If-Modified-Since and If-None-Match header. SEC-1412, SEC-1624.
            if name.eq_ignore_ascii_case(HEADER_IF_MODIFIED_SINCE)
                || name.eq_ignore_ascii_case(HEADER_IF_NONE_MATCH)
            {
                continue;
            }
            let values = request
                .header_values(name)
                .into_iter()
                .map(ToOwned::to_owned)
                .collect();
            saved.headers.insert(name.to_ascii_lowercase(), values);
        }

        if let Some(parameters) = request.parameters() {
            for (name, value) in parameters {
                saved
                    .parameters
                    .entry(name.to_string())
                    .or_insert_with(Vec::new)
                    .push(value.to_string());
            }
        }

        saved
    }

    /// The context path of the request, if any.
    pub fn get_context_path(&self) -> Option<&str> {
        self.context_path.as_deref()
    }

    /// The path information of the request, if any.
    pub fn get_path_info(&self) -> Option<&str> {
        self.path_info.as_deref()
    }

    /// The query string of the request, if any.
    pub fn get_query_string(&self) -> Option<&str> {
        self.query_string.as_deref()
    }

    /// The request URI of the request.
    pub fn get_request_uri(&self) -> &str {
        &self.request_uri
    }

    /// The full request URL of the request, if any.
    pub fn get_request_url(&self) -> Option<&str> {
        self.request_url.as_deref()
    }

    /// The scheme of the request.
    pub fn get_scheme(&self) -> &str {
        &self.scheme
    }

    /// The server name of the request.
    pub fn get_server_name(&self) -> &str {
        &self.server_name
    }

    /// The server port of the request.
    pub fn get_server_port(&self) -> u16 {
        self.server_port
    }

    /// The servlet path of the request, if any.
    pub fn get_servlet_path(&self) -> Option<&str> {
        self.servlet_path.as_deref()
    }

    /// The names of the request parameters.
    pub fn get_parameter_names(&self) -> Vec<String> {
        self.parameters.keys().cloned().collect()
    }

    /// Creates a builder for `DefaultSavedRequest`.
    pub fn builder() -> DefaultSavedRequestBuilder {
        DefaultSavedRequestBuilder::default()
    }

    fn create_query_string(&self) -> String {
        match self.matching_request_parameter_name.as_deref() {
            None => self.query_string.clone().unwrap_or_default(),
            Some(name) => {
                let query_string = match self.query_string.as_deref() {
                    Some(qs) if !qs.is_empty() => qs,
                    _ => return name.to_string(),
                };
                let mut pairs: Vec<String> = query_string
                    .split('&')
                    .filter(|pair| !pair.is_empty())
                    .map(ToOwned::to_owned)
                    .collect();
                pairs.retain(|pair| !pair.starts_with(&format!("{name}=")));
                pairs.push(name.to_string());
                pairs.join("&")
            }
        }
    }
}

impl SavedRequest for DefaultSavedRequest {
    /// Indicates the URL that the user agent used for this request.
    fn get_redirect_url(&self) -> String {
        let query_string = self.create_query_string();
        let scheme = self.scheme.to_lowercase();
        let mut url = format!("{scheme}://{}", self.server_name);
        if (scheme == "http" && self.server_port != 80)
            || (scheme == "https" && self.server_port != 443)
        {
            url.push(':');
            url.push_str(&self.server_port.to_string());
        }
        url.push_str(&self.request_uri);
        if !query_string.is_empty() {
            url.push('?');
            url.push_str(&query_string);
        }
        url
    }

    fn get_cookies(&self) -> Vec<Cookie> {
        self.cookies.clone()
    }

    fn get_method(&self) -> &str {
        self.method.as_str()
    }

    fn get_header_values(&self, name: &str) -> Vec<&str> {
        self.headers
            .get(&name.to_ascii_lowercase()) // Option<&Vec<String>>
            .map(|v| v.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    fn get_header_names(&self) -> Vec<&str> {
        self.headers.keys().map(|s| s.as_str()).collect()
    }

    fn get_locales(&self) -> Vec<Locale> {
        self.locales.to_vec()
    }

    fn get_parameter_values(&self, name: &str) -> Option<Vec<&str>> {
        self.parameters
            .get(name)
            .map(|v| v.as_slice())
            .map(|v| v.iter().map(|s| s.as_str()).collect())
    }

    fn get_parameter_map(&self) -> HashMap<String, Vec<String>> {
        self.parameters.clone()
    }
}

impl fmt::Debug for DefaultSavedRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DefaultSavedRequest")
            .field("cookies", &self.cookies.len())
            .field("locales", &self.locales)
            .field("headers", &self.headers)
            .field("parameters", &self.parameters)
            .field("method", &self.method)
            .field("query_string", &self.query_string)
            .field("request_uri", &self.request_uri)
            .field("scheme", &self.scheme)
            .field("server_name", &self.server_name)
            .field("server_port", &self.server_port)
            .finish()
    }
}

/// Builder for `DefaultSavedRequest`.
pub struct DefaultSavedRequestBuilder {
    cookies: Vec<Cookie>,
    locales: Vec<Locale>,
    headers: HashMap<String, Vec<String>>,
    parameters: HashMap<String, Vec<String>>,
    context_path: Option<String>,
    method: Option<String>,
    path_info: Option<String>,
    query_string: Option<String>,
    request_uri: Option<String>,
    request_url: Option<String>,
    scheme: Option<String>,
    server_name: Option<String>,
    servlet_path: Option<String>,
    server_port: u16,
    matching_request_parameter_name: Option<String>,
}

impl Default for DefaultSavedRequestBuilder {
    fn default() -> Self {
        Self {
            cookies: Vec::new(),
            locales: Vec::new(),
            headers: HashMap::new(),
            parameters: HashMap::new(),
            context_path: None,
            method: None,
            path_info: None,
            query_string: None,
            request_uri: None,
            request_url: None,
            scheme: None,
            server_name: None,
            servlet_path: None,
            server_port: 80,
            matching_request_parameter_name: None,
        }
    }
}

impl DefaultSavedRequestBuilder {
    pub fn build(&mut self) -> DefaultSavedRequest {
        let method = self.method.clone().unwrap_or_else(|| "GET".to_string());
        let mut saved = DefaultSavedRequest {
            cookies: Vec::new(),
            locales: Vec::new(),
            headers: HashMap::new(),
            parameters: HashMap::new(),
            context_path: self.context_path.clone(),
            method,
            path_info: self.path_info.clone(),
            query_string: self.query_string.clone(),
            request_uri: self.request_uri.clone().unwrap_or_default(),
            request_url: self.request_url.clone(),
            scheme: self.scheme.clone().unwrap_or_else(|| "http".to_string()),
            server_name: self.server_name.clone().unwrap_or_default(),
            servlet_path: self.servlet_path.clone(),
            server_port: self.server_port,
            matching_request_parameter_name: self.matching_request_parameter_name.clone(),
        };

        for cookie in &self.cookies {
            saved.cookies.push(cookie.clone());
        }
        saved.locales.extend(self.locales.iter().copied());
        for (name, values) in &self.parameters {
            for value in values {
                saved
                    .parameters
                    .entry(name.clone())
                    .or_insert_with(Vec::new)
                    .push(value.clone());
            }
        }
        self.headers.remove(HEADER_IF_MODIFIED_SINCE);
        self.headers.remove(HEADER_IF_NONE_MATCH);
        for (name, values) in &self.headers {
            for value in values {
                saved
                    .headers
                    .entry(name.to_ascii_lowercase())
                    .or_insert_with(Vec::new)
                    .push(value.clone());
            }
        }
        saved
    }

    pub fn set_cookies(&mut self, cookies: Vec<Cookie>) -> &mut Self {
        self.cookies = cookies;
        self
    }

    pub fn set_locales(&mut self, locales: Option<Vec<Locale>>) -> &mut Self {
        self.locales = locales.unwrap_or_default();
        self
    }

    pub fn set_headers(&mut self, headers: HashMap<String, Vec<String>>) -> &mut Self {
        self.headers = headers;
        self
    }

    pub fn set_parameters(&mut self, parameters: Option<Vec<(&str, &str)>>) -> &mut Self {
        self.parameters.clear();
        if let Some(parameters) = parameters {
            for (name, value) in parameters {
                self.parameters
                    .entry(name.to_string())
                    .or_insert_with(Vec::new)
                    .push(value.to_string());
            }
        }
        self
    }

    pub fn set_context_path(&mut self, context_path: Option<String>) -> &mut Self {
        self.context_path = context_path;
        self
    }

    pub fn set_method(&mut self, method: HttpMethod) -> &mut Self {
        self.method = Some(method.to_string());
        self
    }

    pub fn set_path_info(&mut self, path_info: Option<String>) -> &mut Self {
        self.path_info = path_info;
        self
    }

    pub fn set_query_string(&mut self, query_string: Option<String>) -> &mut Self {
        self.query_string = query_string;
        self
    }

    pub fn set_request_uri(&mut self, request_uri: String) -> &mut Self {
        self.request_uri = Some(request_uri);
        self
    }

    pub fn set_request_url(&mut self, request_url: Option<String>) -> &mut Self {
        self.request_url = request_url;
        self
    }

    pub fn set_scheme(&mut self, scheme: Option<String>) -> &mut Self {
        self.scheme = scheme;
        self
    }

    pub fn set_server_name(&mut self, server_name: Option<String>) -> &mut Self {
        self.server_name = server_name;
        self
    }

    pub fn set_servlet_path(&mut self, servlet_path: Option<String>) -> &mut Self {
        self.servlet_path = servlet_path;
        self
    }

    pub fn set_server_port(&mut self, server_port: u16) -> &mut Self {
        self.server_port = server_port;
        self
    }

    pub fn set_matching_request_parameter_name(
        &mut self,
        matching_request_parameter_name: Option<String>,
    ) -> &mut Self {
        self.matching_request_parameter_name = matching_request_parameter_name;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::DefaultSavedRequest;
    use crate::web::savedrequest::SavedRequest;

    #[test]
    fn builder_builds_redirect_url() {
        let mut builder = DefaultSavedRequest::builder();
        let saved = builder
            .set_scheme(Some("https".to_string()))
            .set_server_name(Some("example.com".to_string()))
            .set_request_uri("/app/page".to_string())
            .set_query_string(Some("a=1&b=2".to_string()))
            .set_server_port(443)
            .set_method(next_web_core::http::HttpMethod::GET)
            .build();
        assert_eq!(
            saved.get_redirect_url(),
            "https://example.com/app/page?a=1&b=2"
        );
    }

    #[test]
    fn builder_defaults_method_to_get() {
        let mut builder = DefaultSavedRequest::builder();
        let saved = builder
            .set_scheme(Some("http".to_string()))
            .set_server_name(Some("localhost".to_string()))
            .set_request_uri("/".to_string())
            .set_server_port(8080)
            .build();
        assert_eq!(saved.get_method(), "GET");
        assert_eq!(saved.get_redirect_url(), "http://localhost:8080/");
    }

    #[test]
    fn matching_request_parameter_is_kept_in_redirect_url() {
        let mut builder = DefaultSavedRequest::builder();
        let saved = builder
            .set_scheme(Some("http".to_string()))
            .set_server_name(Some("localhost".to_string()))
            .set_request_uri("/login".to_string())
            .set_query_string(Some("continue=/app".to_string()))
            .set_server_port(80)
            .set_matching_request_parameter_name(Some("continue".to_string()))
            .build();
        assert_eq!(saved.get_redirect_url(), "http://localhost/login?continue");
    }

    #[test]
    fn parameters_are_kept() {
        let mut builder = DefaultSavedRequest::builder();
        let saved = builder
            .set_scheme(Some("http".to_string()))
            .set_server_name(Some("localhost".to_string()))
            .set_request_uri("/".to_string())
            .set_parameters(Some(vec![("name", "value")]))
            .build();
        assert_eq!(saved.get_parameter_values("name"), Some(vec!["value"]));
        assert!(saved.get_parameter_map().contains_key("name"));
    }
}
