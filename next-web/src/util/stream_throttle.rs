use std::time::Duration;

use axum::{body::Bytes, BoxError};
use futures::{Stream, StreamExt};
use tokio::sync::mpsc;
use tokio::time::Instant;

const MIN_THROTTLE_SLICE: usize = 512;
const MAX_THROTTLE_SLICE: usize = 16 * 1024;
const CHANNEL_SIZE: usize = 16;

/// Wraps an arbitrary byte stream with a soft rate limiter.
///
/// The limiter keeps the average throughput near `target_rate`, while a small
/// deterministic jitter avoids a perfectly uniform pattern.
pub fn throttle_byte_stream<S>(
    source: S,
    target_rate: usize,
) -> impl Stream<Item = Result<Bytes, BoxError>>
where
    S: Stream<Item = Result<Bytes, BoxError>> + Send + 'static,
{
    let (tx, mut rx) = mpsc::channel(CHANNEL_SIZE);

    tokio::spawn(async move {
        let mut source = Box::pin(source);
        let mut limiter = StreamRateLimiter::new(target_rate);

        while let Some(item) = source.next().await {
            match item {
                Ok(chunk) => {
                    if chunk.is_empty() {
                        continue;
                    }

                    let mut offset = 0;
                    while offset < chunk.len() {
                        let allowed = limiter.acquire_budget(chunk.len() - offset).await;
                        let end = offset + allowed;

                        if tx.send(Ok(chunk.slice(offset..end))).await.is_err() {
                            return;
                        }

                        offset = end;
                    }
                }
                Err(error) => {
                    let _ = tx.send(Err(error)).await;
                    return;
                }
            }
        }
    });

    async_stream::stream! {
        while let Some(item) = rx.recv().await {
            yield item;
        }
    }
}

struct StreamRateLimiter {
    rate_per_sec: f64,
    available_tokens: f64,
    max_burst_tokens: f64,
    preferred_slice: usize,
    last_refill_at: Instant,
    next_jitter_at: Instant,
    jitter_factor: f64,
    state: u64,
}

impl StreamRateLimiter {
    fn new(rate_per_sec: usize) -> Self {
        let rate_per_sec = rate_per_sec.max(1);
        let preferred_slice = (rate_per_sec / 12).clamp(MIN_THROTTLE_SLICE, MAX_THROTTLE_SLICE);
        let max_burst_tokens = ((rate_per_sec as f64) * 0.35).max((preferred_slice * 2) as f64);
        let now = Instant::now();

        Self {
            rate_per_sec: rate_per_sec as f64,
            available_tokens: max_burst_tokens.min(preferred_slice as f64),
            max_burst_tokens,
            preferred_slice,
            last_refill_at: now,
            next_jitter_at: now,
            jitter_factor: 1.0,
            state: now.elapsed().as_nanos() as u64 ^ rate_per_sec as u64 ^ 0x9E3779B97F4A7C15,
        }
    }

    async fn acquire_budget(&mut self, desired: usize) -> usize {
        let desired = desired.max(1);
        let minimum_useful_budget = desired.min(self.preferred_slice).max(1);

        loop {
            self.refill();

            let available = self.available_tokens.floor() as usize;
            if available >= minimum_useful_budget || available >= desired {
                let granted = desired.min(available.max(1));
                self.available_tokens -= granted as f64;
                return granted;
            }

            let effective_rate = (self.rate_per_sec * self.jitter_factor).max(1.0);
            let missing = (minimum_useful_budget as f64 - self.available_tokens).max(1.0);
            let wait_seconds = (missing / effective_rate).min(0.25);
            tokio::time::sleep(Duration::from_secs_f64(wait_seconds)).await;
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        self.refresh_jitter(now);

        let elapsed = now.duration_since(self.last_refill_at).as_secs_f64();
        if elapsed <= 0.0 {
            return;
        }

        self.available_tokens = (self.available_tokens
            + elapsed * self.rate_per_sec * self.jitter_factor)
            .min(self.max_burst_tokens);
        self.last_refill_at = now;
    }

    fn refresh_jitter(&mut self, now: Instant) {
        if now < self.next_jitter_at {
            return;
        }

        let ratio = self.next_unit_interval();
        self.jitter_factor = 0.92 + ratio * 0.16;

        let interval_ms = 90 + (self.next_unit_interval() * 150.0) as u64;
        self.next_jitter_at = now + Duration::from_millis(interval_ms);
    }

    fn next_unit_interval(&mut self) -> f64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;

        let value = self.state >> 11;
        value as f64 / ((1u64 << 53) as f64)
    }
}
