use std::collections::HashMap;
use std::str::FromStr;
use std::sync::LazyLock;

use axum::body::Bytes;
use axum::http::{HeaderMap, HeaderName};
use axum::BoxError;
use axum::{body::Body, response::Response};
use futures::stream::BoxStream;
use futures::{stream, StreamExt, TryStreamExt};
use next_web_core::traits::stream::into_response_stream::IntoRespnoseStream;
use reqwest::{header, Client};
use reqwest::{Method, StatusCode};
use tracing::error;

use crate::util::{local_date_time::LocalDateTime, stream_throttle::throttle_byte_stream};

pub static GLOBAL_CLIENT: LazyLock<Client> = LazyLock::new(Client::new);

pub struct NetworkFileStream {
    url: String,
    method: Box<str>,
    headers: Option<HashMap<String, String>>,
}

impl NetworkFileStream {
    pub fn new<S, S1>(url: S, method: S1, headers: Option<HashMap<String, String>>) -> Self
    where
        S: Into<String>,
        S1: Into<Box<str>>,
    {
        let url = url.into();
        let method = method.into();

        assert!(method.as_ref() == "GET" || method.as_ref() == "POST");
        assert!(url.starts_with("http://") || url.starts_with("https://"));

        Self {
            url,
            method,
            headers,
        }
    }
}

impl IntoRespnoseStream for NetworkFileStream {
    fn into_response_stream(self, target_rate: usize) -> Response {
        let url = self.url.clone();

        let mut headers = HeaderMap::new();
        if let Some(header_map) = self.headers {
            header_map.into_iter().for_each(|(key, value)| {
                if let (Ok(name), Ok(value)) = (HeaderName::from_str(&key), value.parse()) {
                    headers.insert(name, value);
                }
            });
        }

        // Parse the method once, up front, so the async block captures owned data.
        let method = Method::from_bytes(self.method.as_bytes()).unwrap_or(Method::GET);
        let request_url = self.url;

        // Build the stream without `async_stream`.
        //
        // `stream::once` produces a single future that resolves to
        // `Result<inner_stream, BoxError>`. `try_flatten_stream` then:
        //   * yields the error once if the outer future fails, or
        //   * flattens the inner `bytes_stream` and forwards its items.
        let source: BoxStream<'static, Result<Bytes, BoxError>> = stream::once(async move {
            let resp = GLOBAL_CLIENT
                .request(method, request_url)
                .headers(headers)
                .send()
                .await
                .map_err(|error| {
                    error!("Error sending request: {}", error);
                    Box::new(error) as BoxError
                })?;

            if !resp.status().is_success() {
                return Err(Box::new(std::io::Error::other(format!(
                    "Http status error: {}",
                    resp.status()
                ))) as BoxError);
            }

            Ok(resp
                .bytes_stream()
                .map_err(|error| Box::new(error) as BoxError)
                .boxed())
        })
        .try_flatten()
        .boxed();

        let header_name = format!(
            "attachment; filename=\"{}\"",
            url.split('/')
                .filter(|s| !s.is_empty())
                .last()
                .unwrap_or(LocalDateTime::now().as_str())
        );

        let body = if target_rate == 0 {
            Body::from_stream(source)
        } else {
            Body::from_stream(throttle_byte_stream(source, target_rate))
        };

        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/octet-stream")
            .header(header::CONTENT_DISPOSITION, header_name)
            .body(body)
            .unwrap()
    }
}

impl NetworkFileStream {
    /// Creates a new `NetworkFileStreamBuilder`.
    pub fn builder() -> NetworkFileStreamBuilder {
        NetworkFileStreamBuilder::default()
    }
}

#[derive(Default)]
pub struct NetworkFileStreamBuilder {
    url: Option<String>,
    method: Option<Box<str>>,
    headers: Option<HashMap<String, String>>,
}

impl NetworkFileStreamBuilder {
    /// Sets the `url` field on the builder.
    pub fn url<T>(mut self, value: T) -> Self
    where
        T: Into<String>,
    {
        self.url = Some(value.into());
        self
    }

    /// Sets the `method` field on the builder.
    pub fn method<T>(mut self, value: T) -> Self
    where
        T: Into<Box<str>>,
    {
        self.method = Some(value.into());
        self
    }

    /// Sets the `headers` field on the builder.
    pub fn headers(mut self, value: HashMap<String, String>) -> Self {
        self.headers = Some(value);
        self
    }

    /// Builds a `NetworkFileStream` instance.
    pub fn build(self) -> Result<NetworkFileStream, String> {
        self.url
            .as_ref()
            .map(|url| assert!(url.starts_with("http://") || url.starts_with("https://")));
        self.method
            .as_ref()
            .map(|method| assert!(method.as_ref() == "GET" || method.as_ref() == "POST"));

        Ok(NetworkFileStream {
            url: self
                .url
                .ok_or_else(|| "field `url` is required".to_string())?,
            method: self
                .method
                .ok_or_else(|| "field `method` is required".to_string())?,
            headers: self.headers,
        })
    }
}
