//! Load balancing and upstream failures.

mod support;

use support::{Behavior, MockUpstream, TestGateway, get, request};

#[test]
fn round_robin_spreads_requests_across_two_upstreams() {
    let first = MockUpstream::start(Behavior::Echo);
    let second = MockUpstream::start(Behavior::Echo);

    let template = format!(
        "\
- id: users
  uri: \"lb://user-service\"
  predicates:
    - Path=/users/**
  upstreams:
    - addr: \"{}\"
    - addr: \"{}\"
",
        first.addr, second.addr
    );
    let gateway = TestGateway::start(&template);

    for _ in 0..4 {
        let response = get(gateway.addr, "/users", &[]);
        assert_eq!(response.status, 200, "{}", response.text());
    }

    assert_eq!(first.request_count(), 2, "first backend should serve two");
    assert_eq!(second.request_count(), 2, "second backend should serve two");
}

#[test]
fn unreachable_upstream_returns_502() {
    // A backend that accepts the connection and closes it immediately.
    let upstream = MockUpstream::start(Behavior::Close);
    let template = format!(
        "\
- id: broken
  uri: \"http://{}\"
  predicates:
    - Path=/broken/**
",
        upstream.addr
    );
    let gateway = TestGateway::start(&template);

    let response = get(gateway.addr, "/broken", &[]);

    assert_eq!(response.status, 502, "{}", response.text());
    assert!(
        response.text().contains("\"status\":502"),
        "{}",
        response.text()
    );
}

#[test]
fn a_weighted_route_prefers_the_heavier_backend() {
    let heavy = MockUpstream::start(Behavior::Echo);
    let light = MockUpstream::start(Behavior::Echo);

    let template = format!(
        "\
- id: users
  uri: \"lb://user-service\"
  predicates:
    - Path=/users/**
  upstreams:
    - addr: \"{}\"
      weight: 3
    - addr: \"{}\"
      weight: 1
",
        heavy.addr, light.addr
    );
    let gateway = TestGateway::start(&template);

    for _ in 0..8 {
        let response = get(gateway.addr, "/users", &[]);
        assert_eq!(response.status, 200);
    }

    assert_eq!(heavy.request_count(), 6);
    assert_eq!(light.request_count(), 2);
}

#[test]
fn a_weight_group_delivers_the_configured_ratio() {
    let heavy = MockUpstream::start(Behavior::Echo);
    let light = MockUpstream::start(Behavior::Echo);

    let template = format!(
        "\
- id: heavy
  uri: \"http://{}\"
  order: 10
  predicates:
    - Path=/weighted/**
    - Weight=canary, 8
- id: light
  uri: \"http://{}\"
  order: 20
  predicates:
    - Path=/weighted/**
    - Weight=canary, 2
",
        heavy.addr, light.addr
    );
    let gateway = TestGateway::start(&template);

    for _ in 0..200 {
        let response = get(gateway.addr, "/weighted", &[]);
        assert_eq!(response.status, 200);
    }

    let total = heavy.request_count() + light.request_count();
    assert_eq!(total, 200);
    // 8:2 means about 160 for the heavy route. The bug this guards against
    // (drawing a bucket per route) produced about 164:36 -> 90:10, which is
    // outside this window on the light side.
    assert!(
        (30..=55).contains(&light.request_count()),
        "an 8:2 group must send about 20% to the light route, got {} of {total}",
        light.request_count()
    );
}

#[test]
fn the_upstream_receives_the_request_target() {
    let upstream = MockUpstream::start(Behavior::Echo);
    let template = format!(
        "\
- id: users
  uri: \"http://{}\"
  predicates:
    - Path=/users/**
",
        upstream.addr
    );
    let gateway = TestGateway::start(&template);

    let response = request(gateway.addr, "POST", "/users/1?x=1", &[], Some(b"body"));

    assert_eq!(response.status, 200);
    let recorded = upstream.last_request().expect("upstream request");
    assert_eq!(recorded.method, "POST");
    assert_eq!(recorded.target, "/users/1?x=1");
    assert_eq!(recorded.body, b"body");
}
