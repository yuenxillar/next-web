use reqwest::{Client, ClientBuilder, Method, RequestBuilder, Url};
use serde::{Serialize, de::DeserializeOwned};
use std::{borrow::Cow, time::Duration};

/// A high-level, easy-to-use HTTP client inspired by Next's `RestTemplate`.
///
/// `RestClient` provides builder-style configuration, base URL support,
/// default headers, and convenient "for object" helpers that combine
/// sending a request with deserializing the response body.
///
/// Cloning a `RestClient` is cheap because `reqwest::Client` is internally
/// reference-counted.
#[derive(Clone)]
pub struct RestClient {
    client: Client,
    base_url: Option<String>,
    default_headers: Vec<(String, String)>,
}

impl RestClient {
    /// Creates a new `RestClient` with default settings.
    ///
    /// Defaults include gzip compression and a 30-second timeout.
    ///
    /// # Panics
    ///
    /// Panics if the underlying `reqwest::Client` cannot be built. Use
    /// [`RestClient::builder`] if you want to handle construction errors.
    pub fn new() -> Self {
        Self::builder()
            .build()
            .expect("failed to build default RestClient")
    }

    /// Creates a [`RestClientBuilder`] for custom configuration.
    pub fn builder() -> RestClientBuilder {
        RestClientBuilder::default()
    }

    /// Starts a GET request to the given URL.
    ///
    /// The URL may be absolute (`https://...`) or relative to the configured
    /// base URL.
    pub fn get<U: AsRef<str>>(&self, url: U) -> RequestBuilder {
        self.request(Method::GET, url)
    }

    /// Starts a POST request to the given URL.
    pub fn post<U: AsRef<str>>(&self, url: U) -> RequestBuilder {
        self.request(Method::POST, url)
    }

    /// Starts a PUT request to the given URL.
    pub fn put<U: AsRef<str>>(&self, url: U) -> RequestBuilder {
        self.request(Method::PUT, url)
    }

    /// Starts a DELETE request to the given URL.
    pub fn delete<U: AsRef<str>>(&self, url: U) -> RequestBuilder {
        self.request(Method::DELETE, url)
    }

    /// Starts a PATCH request to the given URL.
    pub fn patch<U: AsRef<str>>(&self, url: U) -> RequestBuilder {
        self.request(Method::PATCH, url)
    }

    /// Starts a HEAD request to the given URL.
    pub fn head<U: AsRef<str>>(&self, url: U) -> RequestBuilder {
        self.request(Method::HEAD, url)
    }

    /// Performs a generic HTTP request and returns a [`RequestBuilder`] for
    /// further configuration.
    ///
    /// # Parameters
    ///
    /// * `method` - The HTTP method (GET, POST, PUT, DELETE, ...).
    /// * `url`    - The request URL, either absolute or relative to the base URL.
    pub fn request<U: AsRef<str>>(&self, method: Method, url: U) -> RequestBuilder {
        let resolved = self.resolve_url(url.as_ref());
        let mut builder = self.client.request(method, resolved.as_ref());
        for (name, value) in &self.default_headers {
            builder = builder.header(name.as_str(), value.as_str());
        }
        builder
    }

    /// Sends a GET request and deserializes the JSON response body.
    ///
    /// Equivalent to Next's `RestTemplate#getForObject`.
    pub async fn get_for_object<T: DeserializeOwned>(
        &self,
        url: &str,
    ) -> Result<T, reqwest::Error> {
        self.get(url)
            .send()
            .await?
            .error_for_status()?
            .json::<T>()
            .await
    }

    /// Sends a POST request with a JSON body and deserializes the JSON response.
    ///
    /// Equivalent to Next's `RestTemplate#postForObject`.
    pub async fn post_for_object<B: Serialize, T: DeserializeOwned>(
        &self,
        url: &str,
        body: &B,
    ) -> Result<T, reqwest::Error> {
        self.post(url)
            .json(body)
            .send()
            .await?
            .error_for_status()?
            .json::<T>()
            .await
    }

    /// Sends a PUT request with a JSON body and deserializes the JSON response.
    pub async fn put_for_object<B: Serialize, T: DeserializeOwned>(
        &self,
        url: &str,
        body: &B,
    ) -> Result<T, reqwest::Error> {
        self.put(url)
            .json(body)
            .send()
            .await?
            .error_for_status()?
            .json::<T>()
            .await
    }

    /// Sends a DELETE request, ignoring the response body.
    pub async fn delete_for_object(&self, url: &str) -> Result<(), reqwest::Error> {
        self.delete(url)
            .send()
            .await?
            .error_for_status()
            .map(|_| ())
    }

    /// Returns the configured base URL, if any.
    pub fn base_url(&self) -> Option<&str> {
        self.base_url.as_deref()
    }

    /// Resolves `url` against the configured base URL.
    ///
    /// * Absolute URLs (`http://` or `https://`) are returned unchanged.
    /// * Relative URLs are concatenated with the base URL, mirroring the
    ///   behavior of Next's `RestTemplate` (the base path is preserved).
    /// * When no base URL is configured, the URL is returned unchanged.
    fn resolve_url<'a>(&'a self, url: &'a str) -> Cow<'a, str> {
        if url.starts_with("http://") || url.starts_with("https://") {
            return Cow::Borrowed(url);
        }

        match &self.base_url {
            Some(base) => {
                // Concatenate the base URL and the relative path so that the
                // base path is preserved. This matches Next's behavior.
                if url.is_empty() {
                    return Cow::Borrowed(base);
                }
                let mut result = String::with_capacity(base.len() + url.len() + 1);
                result.push_str(base.trim_end_matches('/'));
                if !url.starts_with('/') {
                    result.push('/');
                }
                result.push_str(url);

                Cow::Owned(result)
            }
            None => Cow::Borrowed(url),
        }
    }
}

impl Default for RestClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for [`RestClient`].
///
/// Allows customizing the underlying `reqwest::Client`, the base URL, and
/// default headers.
pub struct RestClientBuilder {
    client_builder: ClientBuilder,
    base_url: Option<String>,
    default_headers: Vec<(String, String)>,
}

impl RestClientBuilder {
    /// Sets the overall request timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.client_builder = self.client_builder.timeout(timeout);
        self
    }

    /// Sets the connection timeout.
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.client_builder = self.client_builder.connect_timeout(timeout);
        self
    }

    // /// Enables or disables gzip compression.
    // pub fn gzip(mut self, enabled: bool) -> Self {
    //     self.client_builder = self.client_builder.gzip(enabled);
    //     self
    // }

    /// Sets the maximum number of idle connections kept per host.
    pub fn max_connections(mut self, max_connections: usize) -> Self {
        self.client_builder = self.client_builder.pool_max_idle_per_host(max_connections);
        self
    }

    /// Sets the default `User-Agent` header.
    pub fn user_agent(mut self, user_agent: &str) -> Self {
        self.client_builder = self.client_builder.user_agent(user_agent);
        self
    }

    /// Sets the base URL used to resolve relative request paths.
    ///
    /// Passing an empty string clears any previously configured base URL.
    ///
    /// # Errors
    ///
    /// Returns a [`Box<dyn std::error::Error>`] if `base_url` is non-empty and cannot be
    /// parsed as a URL.
    pub fn base_url(mut self, base_url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        if base_url.is_empty() {
            self.base_url = None;
            return Ok(self);
        }
        Url::parse(base_url)?;
        self.base_url = Some(base_url.trim_end_matches('/').to_string());
        Ok(self)
    }

    /// Sets default headers that are attached to every request.
    ///
    /// Header order is preserved, and duplicate names are allowed.
    pub fn default_headers<K, V, I>(mut self, headers: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        self.default_headers = headers
            .into_iter()
            .map(|(name, value)| (name.into(), value.into()))
            .collect();
        self
    }

    /// Builds the [`RestClient`].
    ///
    /// # Errors
    ///
    /// Returns a [`reqwest::Error`] if the underlying client cannot be built.
    pub fn build(self) -> Result<RestClient, reqwest::Error> {
        Ok(RestClient {
            client: self.client_builder.build()?,
            base_url: self.base_url,
            default_headers: self.default_headers,
        })
    }
}

impl Default for RestClientBuilder {
    fn default() -> Self {
        Self {
            client_builder: Client::builder()
                .timeout(Duration::from_secs(30))
                .connect_timeout(Duration::from_secs(10)),
            base_url: None,
            default_headers: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[tokio::test]
    async fn test_get_for_string() {
        let client = RestClient::new();
        let body = client
            .get("https://httpbin.org/get")
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        assert!(!body.is_empty());
    }

    #[tokio::test]
    async fn test_post_for_object() {
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct Echo {
            json: serde_json::Value,
        }

        let client = RestClient::new();
        let data = serde_json::json!({ "message": "Hello" });
        let echo: Echo = client
            .post_for_object("https://httpbin.org/post", &data)
            .await
            .unwrap();
        assert_eq!(echo.json["message"], "Hello");
    }

    #[tokio::test]
    async fn test_with_base_url() {
        let client = RestClient::builder()
            .base_url("https://httpbin.org")
            .unwrap()
            .build()
            .unwrap();

        let body = client
            .get("/get")
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        assert!(!body.is_empty());

        assert_eq!(client.base_url(), Some("https://httpbin.org"));
    }

    #[tokio::test]
    async fn test_base_path_preserved() {
        // Base path "/api" must be preserved when joining a relative path.
        let client = RestClient::builder()
            .base_url("https://httpbin.org/api")
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(
            client.resolve_url("/users"),
            "https://httpbin.org/api/users"
        );
        assert_eq!(client.resolve_url("users"), "https://httpbin.org/api/users");
        assert_eq!(
            client.resolve_url("https://other.example.com/x"),
            "https://other.example.com/x"
        );
    }

    #[tokio::test]
    async fn test_get_for_object() {
        #[derive(Deserialize, Debug)]
        struct GetResponse {
            url: String,
        }

        let client = RestClient::new();
        let resp: GetResponse = client
            .get_for_object("https://httpbin.org/get")
            .await
            .unwrap();
        assert!(resp.url.contains("httpbin.org"));
    }

    #[test]
    fn test_invalid_base_url() {
        assert!(RestClient::builder().base_url("not a url").is_err());
    }
}
