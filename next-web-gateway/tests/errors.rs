//! Error semantics: status mapping, breaker interaction and error bodies.

mod support;

use support::{get, with_upstream, Behavior, MockUpstream, TestGateway};

#[test]
fn upstream_500_passes_through_and_trips_the_breaker() {
    let upstream = MockUpstream::start(Behavior::Respond(500, vec![], b"boom".to_vec()));

    let document = format!(
        "next:
  gateway:
    server:
      bind: \"127.0.0.1:{{proxy}}\"
      admin:
        enabled: true
        bind: \"127.0.0.1:{{admin}}\"
    breakers:
      api:
        failure_threshold: 1
        wait_duration_in_open_state: 30s
    routes:
      - id: api
        uri: \"http://{}\"
        predicates:
          - Path=/api/**
        filters:
          - CircuitBreaker=api
",
        upstream.addr
    );

    // The document carries a top level `breakers` block, so the ports are
    // injected by `start_with` rather than by `TestGateway::start`.
    let gateway = TestGateway::start_with(|proxy, admin| {
        document
            .replace("{proxy}", &proxy.to_string())
            .replace("{admin}", &admin.to_string())
    });

    let first = get(gateway.addr, "/api", &[]);
    assert_eq!(first.status, 500, "the 5xx is passed through: {}", first.text());

    let second = get(gateway.addr, "/api", &[]);
    assert_eq!(
        second.status, 503,
        "the open breaker must reject the second call, got {}",
        second.text()
    );
    assert_eq!(second.header("retry-after"), Some("1"));
}

#[test]
fn gateway_error_body_is_json_and_carries_request_id() {
    let template = "\
- id: ping
  uri: \"http://{upstream}\"
  predicates:
    - Path=/ping/**
";
    let (_upstream, gateway) = with_upstream(Behavior::Echo, template);

    let response = get(gateway.addr, "/missing", &[]);

    assert_eq!(response.status, 404);
    assert_eq!(response.header("content-type"), Some("application/json"));
    let body = response.text();
    assert!(body.contains("\"status\":404"), "{body}");
    assert!(body.contains("\"requestId\""), "{body}");
    assert!(body.contains("\"error\":\"Not Found\""), "{body}");
}

#[test]
fn an_unknown_breaker_id_fails_at_startup() {
    let yaml = "\
next:
  gateway:
    routes:
      - id: api
        uri: \"http://127.0.0.1:1\"
        filters:
          - CircuitBreaker=missing
";
    let gateway = next_web_gateway::Gateway::parse_str(yaml).unwrap();
    let error = gateway.check().unwrap_err().to_string();

    assert!(error.contains("unknown circuit breaker"), "{error}");
    assert!(error.contains("missing"), "{error}");
}

#[test]
fn an_unknown_filter_name_fails_at_startup() {
    let yaml = "\
next:
  gateway:
    routes:
      - id: api
        uri: \"http://127.0.0.1:1\"
        filters:
          - StrpPrefix=1
";
    let gateway = next_web_gateway::Gateway::parse_str(yaml).unwrap();
    let error = gateway.check().unwrap_err().to_string();

    assert!(error.contains("StripPrefix"), "{error}");
}
