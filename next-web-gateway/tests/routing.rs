//! Routing and rewriting behaviour, exercised through the real binary.

mod support;

use support::{Behavior, get, request, with_upstream};

#[test]
fn unmatched_path_returns_404() {
    let template = "\
- id: ping
  uri: \"http://{upstream}\"
  predicates:
    - Path=/ping/**
";
    let (_upstream, gateway) = with_upstream(Behavior::Echo, template);

    let response = get(gateway.addr, "/nope", &[]);

    assert_eq!(response.status, 404, "{}", response.text());
    assert!(
        response.text().contains("\"status\":404"),
        "{}",
        response.text()
    );
}

#[test]
fn order_decides_between_overlapping_routes() {
    let template = "\
- id: first
  uri: \"http://{upstream}\"
  order: 10
  predicates:
    - Path=/api/**
  filters:
    - AddRequestHeader=X-Route-Winner, first
- id: second
  uri: \"http://{upstream}\"
  order: 20
  predicates:
    - Path=/api/**
  filters:
    - AddRequestHeader=X-Route-Winner, second
";
    let (upstream, gateway) = with_upstream(Behavior::Echo, template);

    let response = get(gateway.addr, "/api/users", &[]);

    assert_eq!(response.status, 200);
    let recorded = upstream.last_request().expect("upstream request");
    assert_eq!(
        recorded.header("x-route-winner"),
        Some("first"),
        "the lower order must win"
    );
}

#[test]
fn set_request_host_header_overrides_the_upstream_host() {
    let template = "\
- id: echo
  uri: \"http://{upstream}\"
  predicates:
    - Path=/echo/**
  filters:
    - SetRequestHostHeader=api.internal
";
    let (upstream, gateway) = with_upstream(Behavior::Echo, template);

    let response = request(
        gateway.addr,
        "GET",
        "/echo",
        &[("Host", "client.example")],
        None,
    );

    assert_eq!(response.status, 200);
    let recorded = upstream.last_request().expect("upstream request");
    assert_eq!(
        recorded.header("host"),
        Some("api.internal"),
        "SetRequestHostHeader must win over the default host rewrite"
    );
}

#[test]
fn strip_prefix_and_add_request_headers_reach_the_upstream() {
    let template = "\
- id: users
  uri: \"http://{upstream}\"
  predicates:
    - Path=/api/users/**
  filters:
    - StripPrefix=1
    - AddRequestHeader=X-Gateway, next-web
";
    let (upstream, gateway) = with_upstream(Behavior::Echo, template);

    let response = get(gateway.addr, "/api/users/42?q=1", &[]);

    assert_eq!(response.status, 200);

    let recorded = upstream.last_request().expect("upstream request");
    assert_eq!(recorded.target, "/users/42?q=1");
    assert_eq!(recorded.header("x-gateway"), Some("next-web"));
}

#[test]
fn rewrite_path_keeps_query_string() {
    let template = "\
- id: echo
  uri: \"http://{upstream}\"
  predicates:
    - Path=/echo/**
  filters:
    - RewritePath=/echo/(?<rest>.*), /${rest}
";
    let (upstream, gateway) = with_upstream(Behavior::Echo, template);

    let response = get(gateway.addr, "/echo/a/b?x=2", &[]);

    assert_eq!(response.status, 200);
    let recorded = upstream.last_request().expect("upstream request");
    assert_eq!(recorded.target, "/a/b?x=2");
}

#[test]
fn preserve_host_header_keeps_the_original_host() {
    let template = "\
- id: echo
  uri: \"http://{upstream}\"
  predicates:
    - Path=/echo/**
  filters:
    - PreserveHostHeader
";
    let (upstream, gateway) = with_upstream(Behavior::Echo, template);

    let response = request(
        gateway.addr,
        "GET",
        "/echo",
        &[("Host", "client.example")],
        None,
    );

    assert_eq!(response.status, 200);
    let recorded = upstream.last_request().expect("upstream request");
    assert_eq!(recorded.header("host"), Some("client.example"));
}

#[test]
fn the_upstream_host_defaults_to_the_backend() {
    let template = "\
- id: echo
  uri: \"http://{upstream}\"
  predicates:
    - Path=/echo/**
";
    let (upstream, gateway) = with_upstream(Behavior::Echo, template);

    let response = request(
        gateway.addr,
        "GET",
        "/echo",
        &[("Host", "client.example")],
        None,
    );

    assert_eq!(response.status, 200);
    let recorded = upstream.last_request().expect("upstream request");
    assert_eq!(
        recorded.header("host"),
        Some(upstream.addr.to_string().as_str())
    );
}

#[test]
fn forwarding_headers_are_added() {
    let template = "\
- id: echo
  uri: \"http://{upstream}\"
  predicates:
    - Path=/echo/**
";
    let (upstream, gateway) = with_upstream(Behavior::Echo, template);

    let response = get(gateway.addr, "/echo", &[]);

    assert_eq!(response.status, 200);
    let recorded = upstream.last_request().expect("upstream request");
    assert_eq!(recorded.header("x-forwarded-proto"), Some("http"));
    assert!(recorded.header("x-forwarded-for").is_some());
    assert!(recorded.header("x-forwarded-host").is_some());
}
