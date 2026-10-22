//! Smoke test of the integration harness itself.

mod support;

use support::{get, with_upstream, Behavior};

#[test]
fn a_request_reaches_the_mock_upstream_through_the_gateway() {
    let template = "\
      - id: ping
        uri: \"http://{upstream}\"
        predicates:
          - Path=/ping
";
    let (_upstream, gateway) = with_upstream(
        Behavior::Respond(200, vec![], b"pong".to_vec()),
        template,
    );

    let response = get(gateway.addr, "/ping", &[]);

    assert_eq!(response.status, 200);
    assert_eq!(response.text(), "pong");
}

#[test]
fn the_admin_listener_answers_health_and_metrics() {
    let template = "\
      - id: ping
        uri: \"http://{upstream}\"
        predicates:
          - Path=/ping
";
    let (_upstream, gateway) = with_upstream(
        Behavior::Respond(200, vec![], b"pong".to_vec()),
        template,
    );

    let live = get(gateway.admin, "/health/live", &[]);
    assert_eq!(live.status, 200, "{}", live.text());

    let metrics = get(gateway.admin, "/metrics", &[]);
    assert_eq!(metrics.status, 200);
    assert!(metrics.text().contains("# TYPE gateway_requests_total counter"));

    let routes = get(gateway.admin, "/routes", &[]);
    assert_eq!(routes.status, 200);
    assert!(routes.text().contains("\"id\":\"ping\""), "{}", routes.text());
}
