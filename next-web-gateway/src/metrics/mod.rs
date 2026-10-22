//! Request counters exposed in the Prometheus text exposition format.
//!
//! The registry is deliberately small and dependency free: a fixed histogram,
//! a handful of counters and a gauge, all atomic so a worker thread never
//! blocks.

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::time::Duration;

/// Latency histogram bucket boundaries, in milliseconds.
const BUCKETS: [u64; 11] = [1, 5, 10, 25, 50, 100, 250, 500, 1_000, 2_500, 5_000];

/// One request counter per (route, method, status).
#[derive(Default)]
struct RequestCounter {
    count: u64,
    duration_millis: u64,
    buckets: [u64; BUCKETS.len() + 1],
}

/// The counters of one route, grouped by method.
///
/// A `Vec` of methods keeps the hot path allocation free: the method is looked
/// up by comparing the borrowed `&str`, and only a never-seen method allocates.
#[derive(Default)]
struct RouteMetrics {
    methods: Vec<(Box<str>, HashMap<u16, RequestCounter>)>,
}

impl RouteMetrics {
    /// Counter for `method`/`status`, creating the slot on first use.
    fn counter(&mut self, method: &str, status: u16) -> &mut RequestCounter {
        let mut slot = self
            .methods
            .iter()
            .position(|(name, _)| name.as_ref() == method);

        if slot.is_none() {
            self.methods.push((method.into(), HashMap::new()));
            slot = Some(self.methods.len() - 1);
        }

        self.methods[slot.unwrap_or_default()]
            .1
            .entry(status)
            .or_default()
    }

    /// Sum of every counter of this route.
    fn totals(&self) -> (u64, u64, [u64; BUCKETS.len() + 1]) {
        let mut count = 0_u64;
        let mut duration_millis = 0_u64;
        let mut buckets = [0_u64; BUCKETS.len() + 1];

        for (_, statuses) in &self.methods {
            for counter in statuses.values() {
                count = count.saturating_add(counter.count);
                duration_millis = duration_millis.saturating_add(counter.duration_millis);
                for (index, value) in counter.buckets.iter().enumerate() {
                    buckets[index] = buckets[index].saturating_add(*value);
                }
            }
        }

        (count, duration_millis, buckets)
    }
}

/// Counters and gauges for one gateway process.
#[derive(Default)]
pub struct Metrics {
    requests: Mutex<HashMap<Box<str>, RouteMetrics>>,
    upstream_errors: Mutex<HashMap<(String, String), u64>>,
    circuit_states: Mutex<HashMap<String, u8>>,
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
    inflight: AtomicI64,
}

impl Metrics {
    /// Record one finished request.
    ///
    /// No allocation happens on the steady-state path: the route and method are
    /// looked up through their borrowed `&str`, and the status is a `u16` key.
    pub fn observe_request(&self, route: &str, method: &str, status: u16, elapsed: Duration) {
        let millis = elapsed.as_millis() as u64;
        let mut requests = match self.requests.lock() {
            Ok(requests) => requests,
            Err(poisoned) => poisoned.into_inner(),
        };

        let route_metrics = requests.entry(route.into()).or_default();
        let counter = route_metrics.counter(method, status);

        counter.count = counter.count.saturating_add(1);
        counter.duration_millis = counter.duration_millis.saturating_add(millis);

        let slot = BUCKETS
            .iter()
            .position(|bound| millis <= *bound)
            .unwrap_or(BUCKETS.len());
        counter.buckets[slot] = counter.buckets[slot].saturating_add(1);
    }

    /// Record one upstream failure, `kind` being connect/timeout/reset.
    pub fn observe_upstream_error(&self, route: &str, kind: &str) {
        let mut errors = match self.upstream_errors.lock() {
            Ok(errors) => errors,
            Err(poisoned) => poisoned.into_inner(),
        };

        let counter = errors
            .entry((route.to_owned(), kind.to_owned()))
            .or_insert(0);
        *counter = counter.saturating_add(1);
    }

    /// Track how many requests are currently being proxied.
    pub fn record_inflight(&self, delta: i64) {
        let updated = self
            .inflight
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                Some((value + delta).max(0))
            });

        if updated.is_err() {
            self.inflight.store(0, Ordering::Relaxed);
        }
    }

    /// Record one in-process cache lookup.
    pub fn record_cache_lookup(&self, hit: bool) {
        if hit {
            self.cache_hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.cache_misses.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Total number of finished requests.
    pub fn request_total(&self) -> u64 {
        let requests = match self.requests.lock() {
            Ok(requests) => requests,
            Err(poisoned) => poisoned.into_inner(),
        };

        requests
            .values()
            .map(|route| route.totals().0)
            .fold(0_u64, u64::saturating_add)
    }

    /// Record the state of one breaker: `0` closed, `1` open, `2` half open.
    pub fn set_circuit_state(&self, breaker: &str, state: u8) {
        let mut states = match self.circuit_states.lock() {
            Ok(states) => states,
            Err(poisoned) => poisoned.into_inner(),
        };

        states.insert(breaker.to_owned(), state);
    }

    /// Render the Prometheus text exposition format.
    pub fn render_prometheus(&self) -> String {
        let mut out = String::with_capacity(2048);

        out.push_str("# HELP gateway_requests_total Requests handled by the gateway.\n");
        out.push_str("# TYPE gateway_requests_total counter\n");
        {
            let requests = match self.requests.lock() {
                Ok(requests) => requests,
                Err(poisoned) => poisoned.into_inner(),
            };

            let mut routes: Vec<&str> = requests.keys().map(|key| key.as_ref()).collect();
            routes.sort_unstable();

            for route in routes {
                let Some(route_metrics) = requests.get(route) else {
                    continue;
                };

                let mut methods: Vec<&str> = route_metrics
                    .methods
                    .iter()
                    .map(|(method, _)| method.as_ref())
                    .collect();
                methods.sort_unstable();

                for method in methods {
                    let Some((_, statuses)) = route_metrics
                        .methods
                        .iter()
                        .find(|(name, _)| name.as_ref() == method)
                    else {
                        continue;
                    };

                    let mut codes: Vec<u16> = statuses.keys().copied().collect();
                    codes.sort_unstable();

                    for status in codes {
                        let Some(counter) = statuses.get(&status) else {
                            continue;
                        };
                        out.push_str(&format!(
                            "gateway_requests_total{{route=\"{route}\",method=\"{method}\",status=\"{status}\"}} {}\n",
                            counter.count
                        ));
                    }
                }
            }
        }

        out.push_str("# HELP gateway_request_duration_seconds Request latency.\n");
        out.push_str("# TYPE gateway_request_duration_seconds histogram\n");
        {
            let requests = match self.requests.lock() {
                Ok(requests) => requests,
                Err(poisoned) => poisoned.into_inner(),
            };

            let mut routes: Vec<&str> = requests.keys().map(|key| key.as_ref()).collect();
            routes.sort_unstable();

            for route in routes {
                let Some(route_metrics) = requests.get(route) else {
                    continue;
                };
                let (total, millis, buckets) = route_metrics.totals();

                let mut cumulative = 0_u64;
                for (index, bound) in BUCKETS.iter().enumerate() {
                    cumulative = cumulative.saturating_add(buckets[index]);
                    out.push_str(&format!(
                        "gateway_request_duration_seconds_bucket{{route=\"{route}\",le=\"{bound}\"}} {cumulative}\n"
                    ));
                }

                out.push_str(&format!(
                    "gateway_request_duration_seconds_bucket{{route=\"{route}\",le=\"+Inf\"}} {total}\n"
                ));
                out.push_str(&format!(
                    "gateway_request_duration_seconds_count{{route=\"{route}\"}} {total}\n"
                ));
                out.push_str(&format!(
                    "gateway_request_duration_seconds_sum{{route=\"{route}\"}} {}\n",
                    millis as f64 / 1000.0
                ));
            }
        }

        out.push_str("# HELP gateway_upstream_errors_total Upstream failures.\n");
        out.push_str("# TYPE gateway_upstream_errors_total counter\n");
        {
            let errors = match self.upstream_errors.lock() {
                Ok(errors) => errors,
                Err(poisoned) => poisoned.into_inner(),
            };

            let mut keys: Vec<_> = errors.keys().cloned().collect();
            keys.sort();

            for key in keys {
                let Some(count) = errors.get(&key) else {
                    continue;
                };
                let (route, kind) = &key;
                out.push_str(&format!(
                    "gateway_upstream_errors_total{{route=\"{route}\",kind=\"{kind}\"}} {count}\n"
                ));
            }
        }

        out.push_str(
            "# HELP gateway_circuit_state Circuit breaker state: 0 closed, 1 open, 2 half open.\n",
        );
        out.push_str("# TYPE gateway_circuit_state gauge\n");
        {
            let states = match self.circuit_states.lock() {
                Ok(states) => states,
                Err(poisoned) => poisoned.into_inner(),
            };

            let mut breakers: Vec<_> = states.keys().cloned().collect();
            breakers.sort();

            for breaker in breakers {
                let Some(state) = states.get(&breaker) else {
                    continue;
                };
                out.push_str(&format!(
                    "gateway_circuit_state{{breaker=\"{breaker}\"}} {state}\n"
                ));
            }
        }

        out.push_str("# HELP gateway_inflight_requests Requests currently proxied.\n");
        out.push_str("# TYPE gateway_inflight_requests gauge\n");
        out.push_str(&format!(
            "gateway_inflight_requests {}\n",
            self.inflight.load(Ordering::Relaxed)
        ));

        out.push_str("# HELP gateway_local_cache_hits_total Local response cache hits.\n");
        out.push_str("# TYPE gateway_local_cache_hits_total counter\n");
        out.push_str(&format!(
            "gateway_local_cache_hits_total {}\n",
            self.cache_hits.load(Ordering::Relaxed)
        ));
        out.push_str("# TYPE gateway_local_cache_misses_total counter\n");
        out.push_str(&format!(
            "gateway_local_cache_misses_total {}\n",
            self.cache_misses.load(Ordering::Relaxed)
        ));

        out
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::Metrics;

    #[test]
    fn a_breaker_state_is_exported_as_a_gauge() {
        let metrics = Metrics::default();
        metrics.set_circuit_state("user-service", 1);

        let text = metrics.render_prometheus();

        assert!(
            text.contains("# TYPE gateway_circuit_state gauge"),
            "{text}"
        );
        assert!(
            text.contains("gateway_circuit_state{breaker=\"user-service\"} 1"),
            "{text}"
        );
    }

    #[test]
    fn prometheus_exposition_has_help_type_and_labels() {
        let metrics = Metrics::default();
        metrics.observe_request("users", "GET", 200, Duration::from_millis(12));

        let text = metrics.render_prometheus();
        assert!(
            text.contains("# TYPE gateway_requests_total counter"),
            "{text}"
        );
        assert!(
            text.contains(
                "gateway_requests_total{route=\"users\",method=\"GET\",status=\"200\"} 1"
            ),
            "{text}"
        );
        assert!(
            text.contains("gateway_request_duration_seconds_bucket{route=\"users\",le=\"25\"}"),
            "{text}"
        );
        assert!(
            text.contains("gateway_request_duration_seconds_count{route=\"users\"} 1"),
            "{text}"
        );
        // Without `_sum` Prometheus cannot compute an average.
        assert!(
            text.contains("gateway_request_duration_seconds_sum{route=\"users\"} 0.012"),
            "{text}"
        );
        assert!(text.contains("gateway_inflight_requests 0"), "{text}");
    }

    #[test]
    fn request_counter_is_incremented_once_per_request() {
        let metrics = Metrics::default();
        assert_eq!(metrics.request_total(), 0);

        metrics.observe_request("users", "GET", 200, Duration::from_millis(1));
        assert_eq!(metrics.request_total(), 1);

        metrics.observe_request("users", "GET", 200, Duration::from_millis(1));
        assert_eq!(metrics.request_total(), 2);
    }

    #[test]
    fn inflight_never_goes_negative() {
        let metrics = Metrics::default();
        metrics.record_inflight(-1);
        assert!(
            metrics
                .render_prometheus()
                .contains("gateway_inflight_requests 0")
        );
    }

    #[test]
    fn cache_lookups_are_counted_separately() {
        let metrics = Metrics::default();
        metrics.record_cache_lookup(true);
        metrics.record_cache_lookup(false);

        let text = metrics.render_prometheus();
        assert!(text.contains("gateway_local_cache_hits_total 1"), "{text}");
        assert!(
            text.contains("gateway_local_cache_misses_total 1"),
            "{text}"
        );
    }
}
