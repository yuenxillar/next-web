use std::collections::HashMap;

use pingora::http::RequestHeader;
use tracing::warn;

/// Replace `{name}` placeholders with the captured path variables.
pub(crate) fn substitute_variables(template: &str, variables: &HashMap<String, String>) -> String {
    if variables.is_empty() || !template.contains('{') {
        return template.to_string();
    }

    let mut out = String::with_capacity(template.len());
    let mut rest = template;

    while let Some(start) = rest.find('{') {
        let Some(offset) = rest[start..].find('}') else {
            break;
        };
        let end = start + offset;
        let name = &rest[start + 1..end];

        out.push_str(&rest[..start]);
        match variables.get(name) {
            Some(value) => out.push_str(value),
            // Leave an unknown placeholder untouched so a typo stays visible.
            None => out.push_str(&rest[start..=end]),
        }

        rest = &rest[end + 1..];
    }

    out.push_str(rest);
    out
}

pub(crate) fn set_request_path(request_header: &mut RequestHeader, path: &str, query: Option<&str>) {
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

pub(crate) fn build_path_and_query(path: &str, query: Option<&str>) -> String {
    let normalized_path = normalize_path(path);

    match query.filter(|value| !value.is_empty()) {
        Some(query) => format!("{normalized_path}?{query}"),
        None => normalized_path,
    }
}

pub(crate) fn join_paths(prefix: &str, path: &str) -> String {
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

pub(crate) fn strip_prefix_segments(path: &str, offset: usize) -> String {
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::substitute_variables;

    #[test]
    fn substitutes_known_variables() {
        let mut variables = HashMap::new();
        variables.insert("version".to_string(), "v2".to_string());

        assert_eq!(
            substitute_variables("/{version}/users", &variables),
            "/v2/users"
        );
    }

    #[test]
    fn leaves_unknown_variables_untouched() {
        assert_eq!(
            substitute_variables("/{missing}/users", &HashMap::new()),
            "/{missing}/users"
        );
    }

    #[test]
    fn handles_several_placeholders() {
        let mut variables = HashMap::new();
        variables.insert("a".to_string(), "1".to_string());
        variables.insert("b".to_string(), "2".to_string());

        assert_eq!(substitute_variables("/{a}/{b}", &variables), "/1/2");
    }
}
