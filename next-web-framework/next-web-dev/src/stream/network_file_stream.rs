use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

use axum::body::Bytes;
use axum::http::{HeaderMap, HeaderName};
use axum::BoxError;
use axum::{body::Body, response::Response};
use futures::StreamExt;
use next_web_core::traits::stream::into_response_stream::IntoRespnoseStream;
use once_cell::sync::Lazy;
use reqwest::{header, Client};
use reqwest::{Method, StatusCode};
use tokio::time::Instant;
use tracing::error;

use crate::util::local_date_time::LocalDateTime;

pub static GLOBAL_CLIENT: Lazy<Client> = Lazy::new(|| Client::new());

pub struct NetworkFileStream {
    url: String,
    method: String,
    headers: Option<HashMap<String, String>>,
}

impl NetworkFileStream {
    pub fn new<T>(url: T, method: T, headers: Option<HashMap<String, String>>) -> Self
    where
        T: ToString,
    {
        let url = url.to_string();
        let method = method.to_string();
        Self {
            url,
            method,
            headers,
        }
    }
}

impl IntoRespnoseStream for NetworkFileStream {
    fn into_response_stream(self, target_rate: usize) -> Response {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<Result<Bytes, BoxError>>(100);
        let url = self.url.clone();

        if !(url.starts_with("http://") || url.starts_with("https://")) {
            panic!("Invalid url: {}", url);
        }

        let mut headers = HeaderMap::new();
        if let Some(header_map) = self.headers {
            header_map.into_iter().for_each(|(key, value)| {
                headers.insert(HeaderName::from_str(&key).unwrap(), value.parse().unwrap());
            });
        }

        tokio::spawn(async move {
            let resp = match GLOBAL_CLIENT
                .request(
                    Method::from_bytes(self.method.as_bytes()).unwrap_or(Method::GET),
                    self.url,
                )
                .headers(headers)
                .send()
                .await
                .map_err(|e| {
                    error!("Error sending request: {}", e.to_string());
                    e
                }) {
                Ok(resp) => resp,
                Err(e) => {
                    tx.send(Err(Box::new(e))).await.ok();
                    return;
                }
            };
            if !resp.status().is_success() {
                tx.send(Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Http status error: {}", resp.status()),
                ))))
                .await
                .ok();
                return;
            }

            let mut stream = resp.bytes_stream();
            let mut token_bucket = TokenBucket::new(target_rate);
            while let Some(item) = stream.next().await {
                match item {
                    Ok(chunk) => {
                        let mut remaining = chunk.len();
                        let mut offset = 0;

                        // 处理可能超过配额的大块数据
                        while remaining > 0 {
                            let allowed = token_bucket.available();
                            let to_send = remaining.min(allowed);

                            if to_send > 0 {
                                let slice = chunk.slice(offset..offset + to_send);
                                if tx.send(Ok(slice)).await.is_err() {
                                    break;
                                }
                                offset += to_send;
                                remaining -= to_send;
                                token_bucket.consume(to_send);
                            }

                            // 等待新的令牌
                            if remaining > 0 {
                                token_bucket.refill().await;
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(Box::new(e))).await;
                        break;
                    }
                }
            }
        });

        let header_name = format!(
            "attachment; filename=\"{}\"",
            url.split('/')
                .last()
                .unwrap_or(LocalDateTime::now().as_str())
        );

        let stream = async_stream::stream! {
            while let Some(item) = rx.recv().await {
                 yield item;
            }
        };

        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/octet-stream")
            .header(header::CONTENT_DISPOSITION, header_name)
            .body(Body::from_stream(stream))
            .unwrap()
    }
}

struct TokenBucket {
    capacity: usize,
    tokens: usize,
    last_refill: Instant,
    refill_interval: Duration,
}

impl TokenBucket {
    fn new(rate_per_second: usize) -> Self {
        Self {
            capacity: rate_per_second,
            tokens: rate_per_second, // 初始满桶
            last_refill: Instant::now(),
            refill_interval: Duration::from_secs(1),
        }
    }

    fn available(&self) -> usize {
        self.tokens
    }

    fn consume(&mut self, amount: usize) {
        self.tokens = self.tokens.saturating_sub(amount);
    }

    async fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill);

        if elapsed >= self.refill_interval {
            self.tokens = self.capacity;
            self.last_refill = now;
        } else {
            let remaining = self.refill_interval - elapsed;
            tokio::time::sleep(remaining).await;
            self.tokens = self.capacity;
            self.last_refill = Instant::now();
        }
    }
}
