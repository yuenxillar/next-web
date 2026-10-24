//! Admission control: rate limiting and body size limits.

mod support;

use support::{Behavior, MockUpstream, TestGateway, get, request, with_upstream};

#[test]
fn rate_limiter_returns_429_with_retry_after() {
    let template = "\
- id: limited
  uri: \"http://{upstream}\"
  predicates:
    - Path=/limited/**
  filters:
    - RequestRateLimiter=2
";
    let (_upstream, gateway) = with_upstream(Behavior::Echo, template);

    let statuses: Vec<u16> = (0..6)
        .map(|_| get(gateway.addr, "/limited", &[]).status)
        .collect();

    assert!(
        statuses.contains(&429),
        "expected a 429 among the responses, got {statuses:?}"
    );

    let rejected = get(gateway.addr, "/limited", &[]);
    assert_eq!(rejected.status, 429);
    assert_eq!(rejected.header("retry-after"), Some("1"));
    assert_eq!(rejected.header("x-ratelimit-limit"), Some("2"));
}

#[test]
fn request_size_returns_413_for_an_advertised_large_body() {
    let template = "\
- id: small
  uri: \"http://{upstream}\"
  predicates:
    - Path=/small/**
  filters:
    - RequestSize=10
";
    let (_upstream, gateway) = with_upstream(Behavior::Echo, template);

    let response = request(gateway.addr, "POST", "/small", &[], Some(b"0123456789abc"));

    assert_eq!(response.status, 413, "{}", response.text());
}

#[test]
fn request_size_returns_413_for_chunked_over_limit() {
    let template = "\
- id: small
  uri: \"http://{upstream}\"
  predicates:
    - Path=/small/**
  filters:
    - RequestSize=10
";
    let (_upstream, gateway) = with_upstream(Behavior::Echo, template);

    // No `Content-Length`: only the streaming check can reject this.
    let response = support::post_chunked(
        gateway.addr,
        "/small",
        &[b"0123456789".as_slice(), b"abc".as_slice()],
    );

    assert_eq!(response.status, 413, "{}", response.text());
}

#[test]
fn a_request_within_the_limit_is_proxied() {
    let upstream = MockUpstream::start(Behavior::Echo);
    let template = format!(
        "\
- id: small
  uri: \"http://{}\"
  predicates:
    - Path=/small/**
  filters:
    - RequestSize=100
",
        upstream.addr
    );
    let gateway = TestGateway::start(&template);

    let response = request(gateway.addr, "POST", "/small", &[], Some(b"hello"));

    assert_eq!(response.status, 200, "{}", response.text());
}
