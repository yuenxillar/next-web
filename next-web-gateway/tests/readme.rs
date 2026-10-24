//! The README must list every filter and predicate the parser accepts.

#[test]
fn readme_lists_every_filter() {
    let readme = include_str!("../README.md");

    for name in next_web_gateway::KNOWN_FILTERS {
        assert!(
            readme.contains(&format!("`{name}`")),
            "README does not document the filter `{name}`"
        );
    }
}

#[test]
fn readme_lists_every_predicate() {
    let readme = include_str!("../README.md");

    for name in next_web_gateway::KNOWN_PREDICATES {
        assert!(
            readme.contains(&format!("`{name}`")),
            "README does not document the predicate `{name}`"
        );
    }
}

#[test]
fn readme_documents_the_admin_endpoints() {
    let readme = include_str!("../README.md");

    for endpoint in [
        "/health/live",
        "/health/ready",
        "/metrics",
        "/routes",
        "/version",
    ] {
        assert!(
            readme.contains(endpoint),
            "README does not document the admin endpoint `{endpoint}`"
        );
    }
}
