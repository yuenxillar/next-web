//! Observability: request ids, access logs and metrics.

mod support;

use support::{get, with_upstream, Behavior};

#[test]
fn request_id_is_generated_and_echoed() {
    let template = "\
- id: ping
  uri: \"http://{upstream}\"
  predicates:
    - Path=/ping/**

";
    let (_upstream, gateway) = with_upstream(Behavior::Echo, template);

    // A gateway error response always carries the request id.
    let response = get(gateway.addr, "/nope", &[]);

    assert_eq!(response.status, 404);
    let request_id = response.header("x-request-id").expect("generated id");
    assert!(!request_id.is_empty());
    assert!(
        response.text().contains(request_id),
        "the body must carry the id: {}",
        response.text()
    );
}

#[test]
fn an_incoming_request_id_is_reused() {
    let template = "\
- id: ping
  uri: \"http://{upstream}\"
  predicates:
    - Path=/ping/**

";
    let (_upstream, gateway) = with_upstream(Behavior::Echo, template);

    let response = get(gateway.addr, "/ping", &[("X-Request-Id", "caller-supplied")]);

    assert_eq!(response.status, 200);
    assert_eq!(
        response.header("x-request-id"),
        Some("caller-supplied"),
        "the caller's correlation id must be echoed back"
    );

    // A hostile id is not echoed: it could break the header or the JSON body.
    let rejected = get(gateway.addr, "/ping", &[("X-Request-Id", "a b")]);
    assert_ne!(rejected.header("x-request-id"), Some("a b"));
}

#[test]
fn metrics_endpoint_counts_the_request_once() {
    let template = "\
- id: ping
  uri: \"http://{upstream}\"
  predicates:
    - Path=/ping/**

";
    let (_upstream, gateway) = with_upstream(Behavior::Echo, template);

    for _ in 0..3 {
        assert_eq!(get(gateway.addr, "/ping", &[]).status, 200);
    }

    let metrics = get(gateway.admin, "/metrics", &[]).text();
    let line = metrics
        .lines()
        .find(|line| line.starts_with("gateway_requests_total{route=\"ping\""))
        .unwrap_or_default();

    assert!(line.ends_with(" 3"), "expected exactly 3 requests, got `{line}`");
}

#[test]
fn access_log_has_one_line_per_request() {
    let template = "\
- id: ping
  uri: \"http://{upstream}\"
  predicates:
    - Path=/ping/**

";
    let (_upstream, gateway) = with_upstream(Behavior::Echo, template);

    for _ in 0..2 {
        assert_eq!(get(gateway.addr, "/ping", &[]).status, 200);
    }

    // The gateway logs to stdout; poll until both lines are flushed.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let mut log = String::new();
    while std::time::Instant::now() < deadline {
        log = gateway.stdout();
        if log.lines().filter(|line| line.contains("access")).count() >= 2 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    let access_lines = log.lines().filter(|line| line.contains("access")).count();

    assert_eq!(
        access_lines, 2,
        "expected one access line per request, log was:\n{log}"
    );
    assert!(log.contains("request_id="), "log was:\n{log}");
    assert!(log.contains("route=ping"), "log was:\n{log}");
}

#[test]
fn health_ready_reports_ready() {
    let template = "\
- id: ping
  uri: \"http://{upstream}\"
  predicates:
    - Path=/ping/**

";
    let (_upstream, gateway) = with_upstream(Behavior::Echo, template);

    let ready = get(gateway.admin, "/health/ready", &[]);
    assert_eq!(ready.status, 200, "{}", ready.text());
}
