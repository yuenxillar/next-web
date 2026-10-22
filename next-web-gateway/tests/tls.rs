//! `https://` upstreams are unavailable in this build.
//!
//! pingora is compiled without a TLS backend here, so an `https://` route could
//! only ever answer 502. The configuration is therefore rejected at startup, and
//! this suite pins that behaviour: silently accepting the route and failing every
//! request at runtime is the failure mode this project's rules forbid.

mod support;

use std::process::Command;

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_next-web-gateway"))
}

fn check(document: &str) -> std::process::Output {
    let path = std::env::temp_dir().join(format!(
        "next-web-gateway-tls-{}.yaml",
        support::free_port()
    ));
    std::fs::write(&path, document).expect("write the test configuration");

    binary()
        .args(["--check", "--config"])
        .arg(&path)
        .output()
        .expect("run the binary")
}

#[test]
fn an_https_route_is_rejected_at_startup() {
    let document = "\
next:
  gateway:
    routes:
      - id: secure
        uri: \"https://api.example.com\"
        predicates:
          - Path=/**
";

    let output = check(document);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(
        output.status.code(),
        Some(2),
        "an unusable https route is a configuration error: {stderr}"
    );
    assert!(
        stderr.contains("TLS upstreams are unavailable"),
        "the error must name the missing TLS backend: {stderr}"
    );
}

#[test]
fn plain_http_and_lb_routes_still_load() {
    let document = "\
next:
  gateway:
    routes:
      - id: plain
        uri: \"http://127.0.0.1:9000\"
        predicates:
          - Path=/plain/**
      - id: balanced
        uri: \"lb://users\"
        predicates:
          - Path=/users/**
        upstreams:
          - addr: \"127.0.0.1:9001\"
";

    let output = check(document);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(0), "{stderr}");
}
