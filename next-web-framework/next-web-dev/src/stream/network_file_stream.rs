use axum::body::Bytes;
use axum::BoxError;
use axum::{body::Body, response::Response};
use futures::StreamExt;
use next_web_core::interface::stream::into_response_stream::IntoRespnoseStream;
use once_cell::sync::Lazy;
use reqwest::{header, Client};
use reqwest::{Method, StatusCode};

use crate::util::local_date_time::LocalDateTime;

static CLIENT: Lazy<Client> = Lazy::new(|| Client::new());

pub struct NetworkFileStream {
    pub url: String,
    pub method: String,
}

impl IntoRespnoseStream for NetworkFileStream {
    fn into_response_stream(self, _target_rate: usize) -> Response {
        let (tx, rx) = flume::bounded::<Result<Bytes, BoxError>>(100);
        let url = self.url.clone();
        tokio::spawn(async move {
            if let Ok(resp) = CLIENT
                .request(
                    Method::from_bytes(self.method.as_bytes()).unwrap_or(Method::GET),
                    self.url,
                )
                .send()
                .await
            {
                if resp.status().is_success() {
                    let stream = resp.bytes_stream();
                    stream
                        .for_each(|item| async {
                            tx.send(item.map_err(Into::into)).ok();
                        })
                        .await;
                }
            }
        });

        if rx.is_disconnected() {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::empty())
                .unwrap();
        }

        let stream = futures::stream::iter(rx);
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
            .body(Body::from_stream(stream))
            .unwrap()
    }
}
