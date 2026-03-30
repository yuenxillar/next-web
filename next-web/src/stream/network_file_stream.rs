use std::collections::HashMap;
use std::str::FromStr;

use axum::body::Bytes;
use axum::http::{HeaderMap, HeaderName};
use axum::BoxError;
use axum::{body::Body, response::Response};
use futures::StreamExt;
use next_web_core::traits::stream::into_response_stream::IntoRespnoseStream;
use once_cell::sync::Lazy;
use reqwest::{header, Client};
use reqwest::{Method, StatusCode};
use tracing::error;

use crate::util::{local_date_time::LocalDateTime, stream_throttle::throttle_byte_stream};

pub static GLOBAL_CLIENT: Lazy<Client> = Lazy::new(Client::new);

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
                headers.insert(HeaderName::from_str(&key).unwrap(), value.parse().unwrap());
            });
        }

        let source = async_stream::stream! {
            let resp = match GLOBAL_CLIENT
                .request(
                    Method::from_bytes(self.method.as_bytes()).unwrap_or(Method::GET),
                    self.url,
                )
                .headers(headers)
                .send()
                .await
                .map_err(|error| {
                    error!("Error sending request: {}", error);
                    error
                }) {
                Ok(resp) => resp,
                Err(error) => {
                    yield Err::<Bytes, BoxError>(Box::new(error));
                    return;
                }
            };

            if !resp.status().is_success() {
                yield Err::<Bytes, BoxError>(Box::new(std::io::Error::other(format!(
                    "Http status error: {}",
                    resp.status()
                ))));
                return;
            }

            let mut stream = resp.bytes_stream();
            while let Some(item) = stream.next().await {
                match item {
                    Ok(chunk) => yield Ok::<Bytes, BoxError>(chunk),
                    Err(error) => {
                        yield Err::<Bytes, BoxError>(Box::new(error));
                        return;
                    }
                }
            }
        };

        let header_name = format!(
            "attachment; filename=\"{}\"",
            url.split('/')
                .last()
                .unwrap_or(LocalDateTime::now().as_str())
        );

        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/octet-stream")
            .header(header::CONTENT_DISPOSITION, header_name)
            .body(Body::from_stream(throttle_byte_stream(source, target_rate)))
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
