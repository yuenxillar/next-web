//! Trusted proxies: `X-Forwarded-For` is only honoured behind an ingress.

mod support;

use support::{get, Behavior, MockUpstream, TestGateway};

fn document(upstream: std::net::SocketAddr, trusted: &str, predicate: &str, filters: &str) -> String {
    format!(
        "\
next:
  gateway:
    server:
      bind: \"127.0.0.1:{{proxy}}\"
      admin:
        enabled: true
        bind: \"127.0.0.1:{{admin}}\"
    trusted_proxies: {trusted}
    routes:
      - id: forwarded
        uri: \"http://{upstream}\"
        predicates:
          - {predicate}
{filters}
"
    )
}

fn start(document: String) -> TestGateway {
    TestGateway::start_with(|proxy, admin| {
        document
            .replace("{proxy}", &proxy.to_string())
            .replace("{admin}", &admin.to_string())
    })
}

#[test]
fn a_trusted_proxy_header_selects_the_client() {
    let upstream = MockUpstream::start(Behavior::Echo);
    // The test connects from 127.0.0.1, so declaring loopback trusted makes the
    // forwarded header authoritative.
    let gateway = start(document(
        upstream.addr,
        "['127.0.0.1/32']",
        "XForwardedRemoteAddr=203.0.113.0/24",
        "",
    ));

    let allowed = get(gateway.addr, "/x", &[("X-Forwarded-For", "203.0.113.9")]);
    assert_eq!(
        allowed.status, 200,
        "a client inside the range must match: {}",
        allowed.text()
    );

    let denied = get(gateway.addr, "/x", &[("X-Forwarded-For", "198.51.100.1")]);
    assert_eq!(denied.status, 404, "a client outside the range must not match");
}

#[test]
fn an_untrusted_peer_cannot_spoof_the_header() {
    let upstream = MockUpstream::start(Behavior::Echo);
    // Empty trust list: the header is ignored, the direct peer (127.0.0.1) is
    // used, and it is not inside the declared range.
    let gateway = start(document(
        upstream.addr,
        "[]",
        "XForwardedRemoteAddr=203.0.113.0/24",
        "",
    ));

    let response = get(gateway.addr, "/x", &[("X-Forwarded-For", "203.0.113.9")]);

    assert_eq!(
        response.status, 404,
        "an untrusted peer must not be able to spoof its address"
    );
}

#[test]
fn the_rate_limiter_buckets_per_forwarded_client() {
    let upstream = MockUpstream::start(Behavior::Echo);
    let gateway = start(document(
        upstream.addr,
        "['127.0.0.1/32']",
        "Path=/limited",
        "        filters:\n          - RequestRateLimiter=2\n",
    ));

    // Two requests from one client fill its bucket...
    for _ in 0..2 {
        assert_eq!(
            get(gateway.addr, "/limited", &[("X-Forwarded-For", "203.0.113.1")]).status,
            200
        );
    }
    let blocked = get(gateway.addr, "/limited", &[("X-Forwarded-For", "203.0.113.1")]);
    assert_eq!(
        blocked.status, 429,
        "the third request of one client must be limited"
    );

    // ...while a different client behind the same proxy still gets through.
    let other = get(gateway.addr, "/limited", &[("X-Forwarded-For", "203.0.113.2")]);
    assert_eq!(
        other.status, 200,
        "another client must have its own bucket, otherwise every client behind \
         the ingress shares one limit"
    );
}
