//! Filters that only had unit coverage, exercised on the wire.
//!
//! Each case asserts what the client or the upstream actually sees, so a filter
//! that silently stops applying fails here.

mod support;

use support::{Behavior, MockUpstream, TestGateway, with_upstream};

fn start(behavior: Behavior, filters: &str) -> (MockUpstream, TestGateway) {
    let template = format!(
        "\
- id: filters
  uri: \"http://{{upstream}}\"
  predicates:
    - Path=/f/**
  filters:
{filters}"
    );

    with_upstream(behavior, &template)
}

#[test]
fn secure_headers_also_apply_to_a_gateway_generated_error() {
    // The route matches, then `RequestSize` rejects the request, so the gateway
    // renders the 413 itself; D6 requires the response filters to run on that
    // response too.
    let template = "\
- id: limited
  uri: \"http://{upstream}\"
  predicates:
    - Path=/f/**
  filters:
    - RequestSize=10
    - SecureHeaders
";
    let (_upstream, gateway) = with_upstream(Behavior::Echo, template);

    let response = support::request(gateway.addr, "POST", "/f", &[], Some(b"0123456789ABCDEF"));

    assert_eq!(response.status, 413, "{}", response.text());
    assert_eq!(
        response.header("x-frame-options"),
        Some("DENY"),
        "SecureHeaders must run on gateway generated responses"
    );
    assert_eq!(response.header("x-content-type-options"), Some("nosniff"));
}

#[test]
fn redirect_to_answers_without_proxying() {
    let (upstream, gateway) = start(
        Behavior::Echo,
        "    - RedirectTo=302, https://example.com/new\n",
    );

    let response = support::get(gateway.addr, "/f", &[]);

    assert_eq!(response.status, 302);
    assert_eq!(response.header("location"), Some("https://example.com/new"));
    assert_eq!(
        upstream.request_count(),
        0,
        "a redirect must not reach the upstream"
    );
}

#[test]
fn set_status_replaces_the_upstream_status() {
    let (_upstream, gateway) = start(
        Behavior::Respond(200, vec![], b"body".to_vec()),
        "    - SetStatus=418\n",
    );

    let response = support::get(gateway.addr, "/f", &[]);

    assert_eq!(response.status, 418);
    assert_eq!(response.text(), "body", "the body must be preserved");
}

#[test]
fn request_header_size_rejects_a_fat_request_head() {
    let (upstream, gateway) = start(Behavior::Echo, "    - RequestHeaderSize=64\n");

    let response = support::get(gateway.addr, "/f", &[("X-Padding", &"a".repeat(200))]);

    assert_eq!(
        response.status, 431,
        "an oversized request head must be rejected, got {}",
        response.status
    );
    assert_eq!(upstream.request_count(), 0);
}

#[test]
fn dedupe_response_header_collapses_duplicates() {
    let (_upstream, gateway) = start(
        Behavior::Respond(
            200,
            vec![
                ("X-Dupe".to_owned(), "one".to_owned()),
                ("X-Dupe".to_owned(), "two".to_owned()),
            ],
            b"ok".to_vec(),
        ),
        "    - DedupeResponseHeader=X-Dupe, RETAIN_FIRST\n",
    );

    let response = support::get(gateway.addr, "/f", &[]);

    assert_eq!(response.status, 200);
    assert_eq!(
        response.header("x-dupe"),
        Some("one"),
        "RETAIN_FIRST must keep the first value"
    );
}

#[test]
fn response_header_filters_add_and_remove_values() {
    let (_upstream, gateway) = start(
        Behavior::Respond(
            200,
            vec![("X-Drop-Me".to_owned(), "1".to_owned())],
            b"ok".to_vec(),
        ),
        "    - RemoveResponseHeader=X-Drop-Me\n    - AddResponseHeader=X-Added, yes\n",
    );

    let response = support::get(gateway.addr, "/f", &[]);

    assert_eq!(response.header("x-drop-me"), None);
    assert_eq!(response.header("x-added"), Some("yes"));
}

#[test]
fn rewrite_location_response_header_replaces_the_internal_host() {
    let (_upstream, gateway) = start(
        Behavior::Respond(
            302,
            vec![(
                "Location".to_owned(),
                "http://internal:8080/v1/api/thing".to_owned(),
            )],
            Vec::new(),
        ),
        "    - RewriteLocationResponseHeader=ALWAYS_STRIP\n",
    );

    let response = support::get(gateway.addr, "/f", &[]);

    assert_eq!(response.status, 302);
    let location = response.header("location").expect("Location");
    assert!(
        !location.contains("internal:8080"),
        "the internal host must not leak to the client, got {location}"
    );
    assert!(
        location.ends_with("/api/thing"),
        "the version prefix must be stripped, got {location}"
    );
}

#[test]
fn remove_json_attributes_rewrites_the_body() {
    let (_upstream, gateway) = start(
        Behavior::Respond(
            200,
            vec![("Content-Type".to_owned(), "application/json".to_owned())],
            br#"{"keep":1,"secret":"x"}"#.to_vec(),
        ),
        "    - RemoveJsonAttributesResponseBody=secret\n",
    );

    let response = support::get(gateway.addr, "/f", &[]);

    assert_eq!(response.status, 200);
    assert_eq!(response.text(), r#"{"keep":1}"#);
}

#[test]
fn add_request_parameter_keeps_existing_ones() {
    let (_upstream, gateway) = start(Behavior::Echo, "    - AddRequestParameter=added, 1\n");

    let response = support::get(gateway.addr, "/f?keep=2", &[]);

    assert_eq!(response.status, 200);
    assert!(
        response.text().contains("added=1"),
        "the new parameter must reach the upstream: {}",
        response.text()
    );
    assert!(
        response.text().contains("keep=2"),
        "the existing parameter must survive: {}",
        response.text()
    );
}
