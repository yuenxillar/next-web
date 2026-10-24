//! One wire test per predicate, so every name in `KNOWN_PREDICATES` has evidence
//! that it parses, matches and rejects through the real binary.

mod support;

use support::{Behavior, MockUpstream, TestGateway, get, with_upstream};

/// A single route guarded by `predicate`, pointing at a mock upstream.
fn gateway_for(predicate: &str) -> (MockUpstream, TestGateway) {
    let template = format!(
        "\
- id: guarded
  uri: \"http://{{upstream}}\"
  predicates:
    - {predicate}
"
    );

    with_upstream(Behavior::Echo, &template)
}

#[test]
fn method_matches_only_the_listed_verbs() {
    let (_upstream, gateway) = gateway_for("Method=GET");

    assert_eq!(get(gateway.addr, "/anything", &[]).status, 200);
    assert_eq!(
        support::request(gateway.addr, "POST", "/anything", &[], None).status,
        404,
        "a POST must not match `Method=GET`"
    );
}

#[test]
fn method_accepts_a_list() {
    let (_upstream, gateway) = gateway_for("Method=GET,POST");

    assert_eq!(get(gateway.addr, "/anything", &[]).status, 200);
    assert_eq!(
        support::request(gateway.addr, "POST", "/anything", &[], None).status,
        200
    );
    assert_eq!(
        support::request(gateway.addr, "DELETE", "/anything", &[], None).status,
        404
    );
}

#[test]
fn host_matches_an_exact_name() {
    let (_upstream, gateway) = gateway_for("Host=api.example.com");

    assert_eq!(
        get(gateway.addr, "/anything", &[("Host", "api.example.com")]).status,
        200
    );
    assert_eq!(
        get(gateway.addr, "/anything", &[("Host", "other.example.com")]).status,
        404
    );
}

#[test]
fn host_supports_wildcards() {
    let (_upstream, gateway) = gateway_for("Host=**.example.com");

    assert_eq!(
        get(gateway.addr, "/anything", &[("Host", "a.b.example.com")]).status,
        200
    );
    assert_eq!(
        get(gateway.addr, "/anything", &[("Host", "a.other.com")]).status,
        404
    );
}

#[test]
fn header_requires_a_match() {
    let (_upstream, gateway) = gateway_for("Header=X-Tenant, acme-[0-9]+");

    assert_eq!(
        get(gateway.addr, "/anything", &[("X-Tenant", "acme-42")]).status,
        200
    );
    assert_eq!(
        get(gateway.addr, "/anything", &[("X-Tenant", "other-42")]).status,
        404
    );
    assert_eq!(
        get(gateway.addr, "/anything", &[]).status,
        404,
        "an absent header must not match"
    );
}

#[test]
fn header_without_a_pattern_only_requires_presence() {
    let (_upstream, gateway) = gateway_for("Header=X-Tenant");

    assert_eq!(
        get(gateway.addr, "/anything", &[("X-Tenant", "anything")]).status,
        200
    );
    assert_eq!(get(gateway.addr, "/anything", &[]).status, 404);
}

#[test]
fn cookie_matches_a_value() {
    let (_upstream, gateway) = gateway_for("Cookie=session, abc123");

    assert_eq!(
        get(
            gateway.addr,
            "/anything",
            &[("Cookie", "session=abc123; other=1")]
        )
        .status,
        200
    );
    assert_eq!(
        get(gateway.addr, "/anything", &[("Cookie", "session=zzz")]).status,
        404
    );
}

#[test]
fn query_matches_a_parameter() {
    let (_upstream, gateway) = gateway_for("Query=green");

    assert_eq!(get(gateway.addr, "/anything?green", &[]).status, 200);
    assert_eq!(get(gateway.addr, "/anything?red", &[]).status, 404);
}

#[test]
fn query_matches_a_pattern() {
    let (_upstream, gateway) = gateway_for("Query=version, v[0-9]+");

    assert_eq!(get(gateway.addr, "/anything?version=v2", &[]).status, 200);
    assert_eq!(get(gateway.addr, "/anything?version=x", &[]).status, 404);
}

#[test]
fn remote_addr_matches_the_peer_range() {
    // The test client connects from loopback.
    let (_upstream, gateway) = gateway_for("RemoteAddr=127.0.0.1/32");

    assert_eq!(get(gateway.addr, "/anything", &[]).status, 200);

    let (_upstream, gateway) = gateway_for("RemoteAddr=10.0.0.0/8");
    assert_eq!(
        get(gateway.addr, "/anything", &[]).status,
        404,
        "a loopback peer must not match an unrelated range"
    );
}

#[test]
fn after_matches_a_past_instant() {
    let (_upstream, gateway) = gateway_for("After=2020-01-01T00:00:00Z");
    assert_eq!(get(gateway.addr, "/anything", &[]).status, 200);

    let (_upstream, gateway) = gateway_for("After=2099-01-01T00:00:00Z");
    assert_eq!(get(gateway.addr, "/anything", &[]).status, 404);
}

#[test]
fn before_matches_a_future_instant() {
    let (_upstream, gateway) = gateway_for("Before=2099-01-01T00:00:00Z");
    assert_eq!(get(gateway.addr, "/anything", &[]).status, 200);

    let (_upstream, gateway) = gateway_for("Before=2020-01-01T00:00:00Z");
    assert_eq!(get(gateway.addr, "/anything", &[]).status, 404);
}

#[test]
fn between_matches_a_window() {
    let (_upstream, gateway) = gateway_for("Between=2020-01-01T00:00:00Z, 2099-01-01T00:00:00Z");
    assert_eq!(get(gateway.addr, "/anything", &[]).status, 200);

    let (_upstream, gateway) = gateway_for("Between=2020-01-01T00:00:00Z, 2021-01-01T00:00:00Z");
    assert_eq!(
        get(gateway.addr, "/anything", &[]).status,
        404,
        "a window in the past must not match"
    );
}

#[test]
fn path_matches_ant_patterns_and_captures() {
    let template = "\
- id: captured
  uri: \"http://{upstream}\"
  predicates:
    - Path=/api/{tenant}/items/**
  filters:
    - SetPath=/v2/{tenant}/items
";
    let (_upstream, gateway) = with_upstream(Behavior::Echo, template);

    let response = get(gateway.addr, "/api/acme/items/42", &[]);

    assert_eq!(response.status, 200);
    assert!(
        response.text().contains("/v2/acme/items"),
        "the `{{tenant}}` capture must reach SetPath: {}",
        response.text()
    );
    assert_eq!(get(gateway.addr, "/api/items/42", &[]).status, 404);
}
