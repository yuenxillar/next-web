use reqwest::{Client, ClientBuilder, IntoUrl, Method, Response, Url};
use serde::Serialize;
use std::ops::Deref;
use std::time::Duration;

/// 模仿 Spring RestTemplate 风格的 HTTP 客户端
///
/// 提供高层级、易用的 HTTP 请求方法，支持链式配置和基础 URI。
///
/// # Spring-style REST Client
///
/// Provides high-level, easy-to-use HTTP request methods with builder-style configuration
/// and base URI support, inspired by Spring's RestTemplate.
pub struct RestClient {
    client: Client,
    base_url: Option<String>,
}

impl RestClient {
    /// 使用默认配置创建一个新的 `RestClient`
    ///
    /// 默认启用 gzip 压缩，设置合理的超时时间。
    ///
    /// # Creates a new `RestClient` with default settings
    ///
    /// Enables gzip compression by default and sets reasonable timeout values.
    ///
    /// # Returns
    ///
    /// A new `RestClient` instance.
    pub fn new() -> Self {
        Self::builder().build()
    }

    /// 创建一个 `RestClientBuilder` 用于自定义配置
    ///
    /// # Creates a `RestClientBuilder` for custom configuration
    ///
    /// # Returns
    ///
    /// A `RestClientBuilder` instance.
    pub fn builder() -> RestClientBuilder {
        RestClientBuilder::new()
    }

    /// 执行 GET 请求并返回响应体为字符串
    ///
    /// # Parameters
    ///
    /// * `url` - 请求的 URL（可以是相对路径或完整 URL）
    ///
    /// # Returns
    ///
    /// 成功时返回响应体字符串，失败时返回 `reqwest::Error`。
    ///
    /// # Performs a GET request and returns the response body as a string
    ///
    /// # Parameters
    ///
    /// * `url` - The request URL (can be relative path or full URL)
    ///
    /// # Returns
    ///
    /// The response body as a string on success, or `reqwest::Error` on failure.
    pub async fn get_for_string(&self, url: &str) -> reqwest::Result<String> {
        let full_url = self.resolve_url(url);
        self.client.get(&full_url).send().await?.text().await
    }

    /// 执行 GET 请求并将响应体反序列化为指定类型
    ///
    /// # Type Parameters
    ///
    /// * `T` - 目标类型，必须实现 `serde::de::DeserializeOwned`
    ///
    /// # Parameters
    ///
    /// * `url` - 请求的 URL（可以是相对路径或完整 URL）
    ///
    /// # Returns
    ///
    /// 成功时返回反序列化后的对象，失败时返回 `reqwest::Error`。
    ///
    /// # Performs a GET request and deserializes the response body into the specified type
    ///
    /// # Type Parameters
    ///
    /// * `T` - The target type, must implement `serde::de::DeserializeOwned`
    ///
    /// # Parameters
    ///
    /// * `url` - The request URL (can be relative path or full URL)
    ///
    /// # Returns
    ///
    /// The deserialized object on success, or `reqwest::Error` on failure.
    pub async fn get_for_object<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
    ) -> reqwest::Result<T> {
        let full_url = self.resolve_url(url);
        self.client.get(&full_url).send().await?.json::<T>().await
    }

    /// 执行 POST 请求，发送 JSON 数据并返回响应体为字符串
    ///
    /// # Parameters
    ///
    /// * `url` - 请求的 URL（可以是相对路径或完整 URL）
    /// * `body` - 要序列化为 JSON 并发送的请求体
    ///
    /// # Returns
    ///
    /// 成功时返回响应体字符串，失败时返回 `reqwest::Error`。
    ///
    /// # Performs a POST request with JSON body and returns the response body as a string
    ///
    /// # Parameters
    ///
    /// * `url` - The request URL (can be relative path or full URL)
    /// * `body` - The request body to serialize as JSON
    ///
    /// # Returns
    ///
    /// The response body as a string on success, or `reqwest::Error` on failure.
    pub async fn post_for_string<T: Serialize>(
        &self,
        url: &str,
        body: &T,
    ) -> reqwest::Result<String> {
        let full_url = self.resolve_url(url);
        self.client
            .post(&full_url)
            .json(body)
            .send()
            .await?
            .text()
            .await
    }

    /// 执行 POST 请求，发送 JSON 数据并返回反序列化后的对象
    ///
    /// # Type Parameters
    ///
    /// * `T` - 请求体类型，必须实现 `serde::Serialize`
    /// * `U` - 响应体目标类型，必须实现 `serde::de::DeserializeOwned`
    ///
    /// # Parameters
    ///
    /// * `url` - 请求的 URL（可以是相对路径或完整 URL）
    /// * `body` - 要序列化为 JSON 并发送的请求体
    ///
    /// # Returns
    ///
    /// 成功时返回反序列化后的对象，失败时返回 `reqwest::Error`。
    ///
    /// # Performs a POST request with JSON body and deserializes the response
    ///
    /// # Type Parameters
    ///
    /// * `T` - The request body type, must implement `serde::Serialize`
    /// * `U` - The response body target type, must implement `serde::de::DeserializeOwned`
    ///
    /// # Parameters
    ///
    /// * `url` - The request URL (can be relative path or full URL)
    /// * `body` - The request body to serialize as JSON
    ///
    /// # Returns
    ///
    /// The deserialized response object on success, or `reqwest::Error` on failure.
    pub async fn post_for_object<T: Serialize, U: serde::de::DeserializeOwned>(
        &self,
        url: &str,
        body: &T,
    ) -> reqwest::Result<U> {
        let full_url = self.resolve_url(url);
        self.client
            .post(&full_url)
            .json(body)
            .send()
            .await?
            .json::<U>()
            .await
    }

    /// 执行通用 HTTP 请求
    ///
    /// # Parameters
    ///
    /// * `method` - HTTP 方法（GET, POST, PUT, DELETE 等）
    /// * `url` - 请求的 URL（可以是相对路径或完整 URL）
    ///
    /// # Returns
    ///
    /// 返回 `RequestBuilder` 用于进一步配置请求。
    ///
    /// # Performs a generic HTTP request
    ///
    /// # Parameters
    ///
    /// * `method` - The HTTP method (GET, POST, PUT, DELETE, etc.)
    /// * `url` - The request URL (can be relative path or full URL)
    ///
    /// # Returns
    ///
    /// Returns a `RequestBuilder` for further request configuration.
    pub fn request<U>(&self, method: Method, url: U) -> reqwest::RequestBuilder
    where
        U: IntoUrl + AsRef<str>,
        U: Clone,
    {
        let _url = url.clone();
        let url = match _url.into_url() {
            Ok(u) => u,
            Err(_) => {
                // 如果传入的是相对路径且有 base_url，则解析
                if let Some(base) = &self.base_url {
                    match Url::parse(base) {
                        Ok(mut base_url) => {
                            base_url.set_path(url.as_ref());
                            base_url
                        }
                        Err(_) => return self.client.request(method, url), // 解析失败，直接使用原值
                    }
                } else {
                    // 没有 base_url，直接使用原值
                    return self.client.request(method, url);
                }
            }
        };
        self.client.request(method, url)
    }

    /// 执行 PUT 请求
    ///
    /// # Parameters
    ///
    /// * `url` - 请求的 URL（可以是相对路径或完整 URL）
    /// * `body` - 请求体
    ///
    /// # Returns
    ///
    /// `Response` 对象或错误。
    ///
    /// # Performs a PUT request
    ///
    /// # Parameters
    ///
    /// * `url` - The request URL (can be relative path or full URL)
    /// * `body` - The request body
    ///
    /// # Returns
    ///
    /// The `Response` object or an error.
    pub async fn put<T: Serialize>(&self, url: &str, body: &T) -> reqwest::Result<Response> {
        let full_url = self.resolve_url(url);
        self.client.put(&full_url).json(body).send().await
    }

    /// 执行 DELETE 请求
    ///
    /// # Parameters
    ///
    /// * `url` - 请求的 URL（可以是相对路径或完整 URL）
    ///
    /// # Returns
    ///
    /// `Response` 对象或错误。
    ///
    /// # Performs a DELETE request
    ///
    /// # Parameters
    ///
    /// * `url` - The request URL (can be relative path or full URL)
    ///
    /// # Returns
    ///
    /// The `Response` object or an error.
    pub async fn delete(&self, url: &str) -> reqwest::Result<Response> {
        let full_url = self.resolve_url(url);
        self.client.delete(&full_url).send().await
    }

    /// 解析 URL，结合 base_url（如果存在）
    ///
    /// # Parameters
    ///
    /// * `url` - 要解析的 URL（相对路径或完整 URL）
    ///
    /// # Returns
    ///
    /// 完整的 URL 字符串。
    ///
    /// # Resolves URL by combining with base_url if present
    ///
    /// # Parameters
    ///
    /// * `url` - The URL to resolve (relative path or full URL)
    ///
    /// # Returns
    ///
    /// The complete URL string.
    fn resolve_url(&self, url: &str) -> String {
        if url.starts_with("http://") || url.starts_with("https://") {
            // 如果是完整 URL，直接返回
            url.to_string()
        } else if let Some(base) = &self.base_url {
            // 如果有 base_url，尝试合并
            match Url::parse(base) {
                Ok(mut base_url) => {
                    base_url.set_path(url);
                    base_url.to_string()
                }
                Err(_) => url.to_string(), // base_url 解析失败，返回原值
            }
        } else {
            // 没有 base_url，返回原值（假设是相对路径，但 reqwest 会处理）
            url.to_string()
        }
    }

    /// 获取当前的 base_url
    ///
    /// # Returns
    ///
    /// 如果设置了 base_url，则返回 `Some(&str)`，否则返回 `None`。
    ///
    /// # Get the current base_url
    ///
    /// # Returns
    ///
    /// Returns `Some(&str)` if base_url is set, otherwise `None`.
    pub fn base_url(&self) -> Option<&str> {
        self.base_url.as_deref()
    }
}

/// `RestClient` 的构建器
///
/// 用于配置 `RestClient` 的各种选项，包括基础 URI。
///
/// # Builder for `RestClient`
///
/// Used to configure various options of the `RestClient`, including base URI.
pub struct RestClientBuilder {
    client_builder: ClientBuilder,
    base_url: Option<String>,
}

impl RestClientBuilder {
    /// 创建一个新的 `RestClientBuilder`，使用默认设置
    ///
    /// # Creates a new `RestClientBuilder` with default settings
    ///
    /// # Returns
    ///
    /// A new `RestClientBuilder` instance.
    fn new() -> Self {
        Self {
            client_builder: Client::builder(),
            base_url: None,
        }
    }

    /// 设置客户端的默认超时时间
    ///
    /// # Parameters
    ///
    /// * `timeout` - 超时持续时间
    ///
    /// # Returns
    ///
    /// 自身，用于链式调用。
    ///
    /// # Sets the default timeout for the client
    ///
    /// # Parameters
    ///
    /// * `timeout` - The timeout duration
    ///
    /// # Returns
    ///
    /// Self, for method chaining.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.client_builder = self.client_builder.timeout(timeout);
        self
    }

    /// 设置连接超时时间
    ///
    /// # Parameters
    ///
    /// * `timeout` - 连接超时持续时间
    ///
    /// # Returns
    ///
    /// 自身，用于链式调用。
    ///
    /// # Sets the connect timeout for the client
    ///
    /// # Parameters
    ///
    /// * `timeout` - The connect timeout duration
    ///
    /// # Returns
    ///
    /// Self, for method chaining.
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.client_builder = self.client_builder.connect_timeout(timeout);
        self
    }

    /// 设置是否启用 gzip 压缩
    ///
    /// # Parameters
    ///
    /// * `enabled` - 是否启用
    ///
    /// # Returns
    ///
    /// 自身，用于链式调用。
    ///
    /// # Sets whether gzip compression is enabled
    ///
    /// # Parameters
    ///
    /// * `enabled` - Whether to enable gzip compression
    ///
    /// # Returns
    ///
    /// Self, for method chaining.
    pub fn gzip(mut self, enabled: bool) -> Self {
        self.client_builder = self.client_builder.gzip(enabled);
        self
    }

    /// 设置最大连接数
    ///
    /// # Parameters
    ///
    /// * `max_connections` - 最大连接数
    ///
    /// # Returns
    ///
    /// 自身，用于链式调用。
    ///
    /// # Sets the maximum number of connections
    ///
    /// # Parameters
    ///
    /// * `max_connections` - Maximum number of connections
    ///
    /// # Returns
    ///
    /// Self, for method chaining.
    pub fn max_connections(mut self, max_connections: usize) -> Self {
        self.client_builder = self.client_builder.pool_max_idle_per_host(max_connections);
        self
    }

    /// 设置用户代理字符串
    ///
    /// # Parameters
    ///
    /// * `user_agent` - 用户代理字符串
    ///
    /// # Returns
    ///
    /// 自身，用于链式调用。
    ///
    /// # Sets the user agent string
    ///
    /// # Parameters
    ///
    /// * `user_agent` - The user agent string
    ///
    /// # Returns
    ///
    /// Self, for method chaining.
    pub fn user_agent(mut self, user_agent: &str) -> Self {
        self.client_builder = self.client_builder.user_agent(user_agent);
        self
    }

    /// 设置基础 URI
    ///
    /// 所有相对路径的请求都会基于此基础 URI。
    ///
    /// # Parameters
    ///
    /// * `base_url` - 基础 URI（例如：`https://api.example.com/v1`）
    ///
    /// # Returns
    ///
    /// 自身，用于链式调用。
    ///
    /// # Sets the base URI
    ///
    /// All relative path requests will be based on this base URI.
    ///
    /// # Parameters
    ///
    /// * `base_url` - The base URI (e.g., `https://api.example.com/v1`)
    ///
    /// # Returns
    ///
    /// Self, for method chaining.
    pub fn base_url(mut self, base_url: &str) -> Self {
        self.base_url = Some(base_url.to_string());
        self
    }

    /// 构建 `RestClient` 实例
    ///
    /// # Returns
    ///
    /// 构建好的 `RestClient` 实例。
    ///
    /// # Builds the `RestClient` instance
    ///
    /// # Returns
    ///
    /// The constructed `RestClient` instance.
    pub fn build(self) -> RestClient {
        RestClient {
            client: self
                .client_builder
                .build()
                .expect("Failed to build reqwest client"),
            base_url: self.base_url,
        }
    }
}

impl Clone for RestClient {
    /// 克隆 `RestClient`
    ///
    /// 由于 `reqwest::Client` 内部使用 `Arc`，克隆是轻量级的。
    ///
    /// # Clones the `RestClient`
    ///
    /// Cloning is lightweight as `reqwest::Client` internally uses `Arc`.
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            base_url: self.base_url.clone(),
        }
    }
}

impl Deref for RestClient {
    /// 允许直接通过 `RestClient` 实例访问 `reqwest::Client` 的方法
    ///
    /// # Allows direct access to `reqwest::Client` methods through `RestClient` instance
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_for_string() {
        let client = RestClient::new();
        let result = client.get_for_string("https://httpbin.org/get").await;
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_post_for_object() {
        #[derive(Serialize, serde::Deserialize, PartialEq, Debug)]
        struct Echo {
            json: serde_json::Value,
        }

        let client = RestClient::new();
        let data = serde_json::json!({"message": "Hello"});
        let result: Result<Echo, _> = client
            .post_for_object("https://httpbin.org/post", &data)
            .await;
        assert!(result.is_ok());
        let echo = result.unwrap();
        assert_eq!(echo.json["message"], "Hello");
    }

    #[tokio::test]
    async fn test_with_base_url() {
        let client = RestClient::builder()
            .base_url("https://httpbin.org")
            .build();

        // 使用相对路径
        let result = client.get_for_string("/get").await;
        assert!(result.is_ok());

        // 验证 base_url
        assert_eq!(client.base_url(), Some("https://httpbin.org"));
    }

    #[tokio::test]
    async fn test_mixed_urls() {
        let client = RestClient::builder()
            .base_url("https://httpbin.org/api")
            .build();

        // 相对路径会与 base_url 合并
        let result1 = client.get_for_string("/users").await;
        assert!(result1.is_ok());

        // 完整 URL 不受影响
        let result2 = client.get_for_string("https://httpbin.org/get").await;
        assert!(result2.is_ok());
    }

}
