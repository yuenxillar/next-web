//! A retrying endpoint whose metrics are scraped over HTTP.
//!
//! Run it, call `POST /charge` once, then read `GET /metrics`:
//!
//! ```text
//! cargo run -p next-web-examples --bin retry_metrics
//! curl -X POST http://127.0.0.1:8099/charge
//! curl http://127.0.0.1:8099/metrics
//! ```
//!
//! The first two attempts of `charge` fail and the third succeeds, so the exposition shows one
//! successful block, two failed attempts and the retries the block performed.

use std::sync::{
    Arc,
    atomic::{AtomicU16, Ordering},
};

use axum::{
    Router,
    extract::State,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use next_web_retry::{
    error::retry_error::RetryError,
    retry_callback::with_fn,
    retry_context::{RetryContext, retry_context_constants},
    retry_operations::RetryOperations,
    support::{
        metrics_retry_listener::MetricsRetryListener, retry_metrics::RetryMetrics,
        retry_template::RetryTemplate,
    },
};

/// The reporting name of the retried operation, which becomes the metric label.
const OPERATION: &str = "charge";

/// Shared state: the template to retry with, the registry to report into, and the attempt counter
/// that makes the first two attempts fail.
#[derive(Clone)]
struct AppState {
    template: RetryTemplate,
    metrics: Arc<RetryMetrics>,
    attempts: Arc<AtomicU16>,
}

/// Runs the charge, failing the first two attempts.
async fn charge(State(state): State<AppState>) -> Response {
    let attempts = state.attempts.clone();

    let result = state
        .template
        .execute(with_fn(move |context: &dyn RetryContext| {
            let attempts = attempts.clone();
            Box::pin(async move {
                // The label of the block: a static name, never an item id.
                context.set_attribute(retry_context_constants::NAME, OPERATION.into());

                let attempt = attempts.fetch_add(1, Ordering::Relaxed) + 1;
                if attempt <= 2 {
                    return Err(RetryError::custom("the payment provider is unavailable"));
                }

                Ok::<u16, RetryError>(attempt)
            })
        }))
        .await;

    match result {
        Ok(attempts) => {
            (StatusCode::OK, format!("charged on attempt {attempts}\n")).into_response()
        }
        Err(error) => (
            StatusCode::SERVICE_UNAVAILABLE,
            format!("failed: {error}\n"),
        )
            .into_response(),
    }
}

/// Serves the Prometheus text exposition of the retry metrics.
async fn scrape(State(state): State<AppState>) -> Response {
    (
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        state.metrics.render_prometheus(),
    )
        .into_response()
}

#[tokio::main]
async fn main() {
    let metrics = Arc::new(RetryMetrics::new());
    let template = RetryTemplate::builder()
        .max_attempts(3)
        .fixed_backoff(10)
        .with_listener(MetricsRetryListener::new(metrics.clone()))
        .build();

    let state = AppState {
        template,
        metrics,
        attempts: Arc::new(AtomicU16::new(0)),
    };

    let app = Router::new()
        .route("/charge", post(charge))
        .route("/metrics", get(scrape))
        .with_state(state);

    let address = "127.0.0.1:8099";
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .expect("the example must be able to bind its port");
    println!("serving retry metrics on http://{address}");

    axum::serve(listener, app)
        .await
        .expect("the example server must run");
}
