//! The in-process response cache, exercised on the wire.

mod support;

use support::{get, Behavior, MockUpstream, TestGateway};

fn cache_route(upstream: std::net::SocketAddr) -> String {
    format!(
        "\
next:
  gateway:
    server:
      bind: \"127.0.0.1:{{proxy}}\"
      admin:
        enabled: true
        bind: \"127.0.0.1:{{admin}}\"
    routes:
      - id: cacheable
        uri: \"http://{upstream}\"
        predicates:
          - Path=/cache/**
        filters:
          - LocalResponseCache=60s, 1MB
"
    )
}

#[test]
fn a_cache_hit_returns_the_cached_body() {
    let upstream = MockUpstream::start(Behavior::Respond(
        200,
        vec![("Content-Type".to_string(), "text/plain".to_string())],
        b"cached-body".to_vec(),
    ));
    let document = cache_route(upstream.addr);
    let gateway = TestGateway::start_with(|proxy, admin| {
        document
            .replace("{proxy}", &proxy.to_string())
            .replace("{admin}", &admin.to_string())
    });

    let first = get(gateway.addr, "/cache", &[]);
    let second = get(gateway.addr, "/cache", &[]);

    assert_eq!(first.status, 200, "{}", first.text());
    assert_eq!(first.text(), "cached-body");
    assert_eq!(
        second.status, 200,
        "a cache hit must answer with the cached status, got {}: {}",
        second.status,
        second.text()
    );
    assert_eq!(
        second.text(),
        "cached-body",
        "a cache hit must return the cached body, not an error document"
    );
    assert_eq!(
        upstream.request_count(),
        1,
        "the second request must be served from the cache"
    );
}

#[test]
fn a_response_that_sets_a_cookie_is_not_cached() {
    let upstream = MockUpstream::start(Behavior::Respond(
        200,
        vec![("Set-Cookie".to_string(), "session=secret".to_string())],
        b"personal".to_vec(),
    ));
    let document = cache_route(upstream.addr);
    let gateway = TestGateway::start_with(|proxy, admin| {
        document
            .replace("{proxy}", &proxy.to_string())
            .replace("{admin}", &admin.to_string())
    });

    let first = get(gateway.addr, "/cache", &[]);
    let second = get(gateway.addr, "/cache", &[]);

    assert_eq!(first.status, 200);
    assert_eq!(second.status, 200);
    assert_eq!(
        upstream.request_count(),
        2,
        "a response carrying Set-Cookie must never be replayed from the cache"
    );
}

#[test]
fn a_response_with_an_unsupported_vary_is_not_cached() {
    let upstream = MockUpstream::start(Behavior::Respond(
        200,
        vec![("Vary".to_string(), "X-Tenant".to_string())],
        b"tenant-a".to_vec(),
    ));
    let document = cache_route(upstream.addr);
    let gateway = TestGateway::start_with(|proxy, admin| {
        document
            .replace("{proxy}", &proxy.to_string())
            .replace("{admin}", &admin.to_string())
    });

    assert_eq!(get(gateway.addr, "/cache", &[]).status, 200);
    assert_eq!(get(gateway.addr, "/cache", &[]).status, 200);

    assert_eq!(
        upstream.request_count(),
        2,
        "a response that varies on a header the key does not know must not be cached"
    );
}

#[test]
fn a_204_is_not_reframed_when_a_body_filter_is_configured() {
    let upstream = MockUpstream::start(Behavior::Respond(204, vec![], Vec::new()));
    let document = format!(
        "\
next:
  gateway:
    server:
      bind: \"127.0.0.1:{{proxy}}\"
      admin:
        enabled: true
        bind: \"127.0.0.1:{{admin}}\"
    routes:
      - id: no-content
        uri: \"http://{}\"
        predicates:
          - Path=/nocontent
        filters:
          - RemoveJsonAttributesResponseBody=secret
",
        upstream.addr
    );
    let gateway = TestGateway::start_with(|proxy, admin| {
        document
            .replace("{proxy}", &proxy.to_string())
            .replace("{admin}", &admin.to_string())
    });

    let response = get(gateway.addr, "/nocontent", &[]);

    assert_eq!(response.status, 204);
    assert_eq!(
        response.header("transfer-encoding"),
        None,
        "a 204 must not be reframed as chunked"
    );
    assert!(
        response.text().is_empty(),
        "a 204 must carry no body, got {:?}",
        response.text()
    );
}

#[test]
fn a_head_response_carries_no_body() {
    let upstream = MockUpstream::start(Behavior::Respond(200, vec![], b"payload".to_vec()));
    let document = format!(
        "\
next:
  gateway:
    server:
      bind: \"127.0.0.1:{{proxy}}\"
      admin:
        enabled: true
        bind: \"127.0.0.1:{{admin}}\"
    routes:
      - id: head
        uri: \"http://{}\"
        predicates:
          - Path=/head
",
        upstream.addr
    );
    let gateway = TestGateway::start_with(|proxy, admin| {
        document
            .replace("{proxy}", &proxy.to_string())
            .replace("{admin}", &admin.to_string())
    });

    let response = support::request(gateway.addr, "HEAD", "/head", &[], None);

    assert_eq!(response.status, 200);
    assert!(
        response.text().is_empty(),
        "HEAD must not return a body, got {:?}",
        response.text()
    );
}

#[test]
fn a_no_cache_request_bypasses_the_cache() {
    let upstream = MockUpstream::start(Behavior::Respond(200, vec![], b"fresh".to_vec()));
    let document = cache_route(upstream.addr);
    let gateway = TestGateway::start_with(|proxy, admin| {
        document
            .replace("{proxy}", &proxy.to_string())
            .replace("{admin}", &admin.to_string())
    });

    // Warm the cache.
    assert_eq!(get(gateway.addr, "/cache", &[]).text(), "fresh");
    assert_eq!(upstream.request_count(), 1);

    let revalidated = get(gateway.addr, "/cache", &[("Cache-Control", "no-cache")]);

    assert_eq!(
        revalidated.status, 200,
        "a no-cache request must be revalidated, not answered with a synthetic 304"
    );
    assert_eq!(revalidated.text(), "fresh");
    assert_eq!(
        upstream.request_count(),
        2,
        "the no-cache request must reach the upstream"
    );
}
