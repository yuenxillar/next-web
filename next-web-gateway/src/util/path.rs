use pingora::http::RequestHeader;
use tracing::warn;

pub fn set_request_path(request_header: &mut RequestHeader, path: &str, query: Option<&str>) {
    let path_and_query = build_path_and_query(path, query);

    // Rebuild the URI from a normalized path/query pair and keep the request mutable in place.
    match path_and_query.parse() {
        Ok(uri) => request_header.set_uri(uri),
        Err(error) => warn!(
            target: "gateway_filter",
            "Failed to parse rewritten request URI `{}`: {}",
            path_and_query,
            error
        ),
    }
}

pub fn build_path_and_query(path: &str, query: Option<&str>) -> String {
    let normalized_path = normalize_path(path);

    match query.filter(|value| !value.is_empty()) {
        Some(query) => format!("{normalized_path}?{query}"),
        None => normalized_path,
    }
}

pub fn join_paths(prefix: &str, path: &str) -> String {
    let normalized_prefix = normalize_path(prefix);
    let normalized_path = normalize_path(path);

    if normalized_prefix == "/" {
        return normalized_path;
    }

    if normalized_path == "/" {
        return normalized_prefix;
    }

    format!(
        "{}/{}",
        normalized_prefix.trim_end_matches('/'),
        normalized_path.trim_start_matches('/')
    )
}

pub fn strip_prefix_segments(path: &str, offset: usize) -> String {
    if offset == 0 {
        return normalize_path(path);
    }

    let trailing_slash = path.ends_with('/') && path.len() > 1;
    let segments = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();

    if offset >= segments.len() {
        return "/".to_string();
    }

    let mut stripped = format!("/{}", segments[offset..].join("/"));
    if trailing_slash && stripped != "/" {
        stripped.push('/');
    }
    stripped
}

fn normalize_path(path: &str) -> String {
    let trimmed = path.trim();

    if trimmed.is_empty() || trimmed == "/" {
        return "/".to_string();
    }

    if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{trimmed}")
    }
}
