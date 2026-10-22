//! `Path` predicate: ANT style patterns with `{variable}` captures.

use std::collections::HashMap;

use regex::Regex;

use super::route_predicate::RoutePredicate;

/// A compiled path pattern plus the names of its capture groups.
#[derive(Debug, Clone)]
pub struct PathPattern {
    /// The pattern as written in the configuration.
    pub pattern: String,
    /// Compiled regular expression, anchored at both ends.
    pub regex: Regex,
    /// Capture group names, in group order.
    pub variables: Vec<String>,
}

impl PathPattern {
    /// Whether a request path matches, ignoring any query string.
    pub fn is_match(&self, path: &str) -> bool {
        self.regex.is_match(path)
    }

    /// Extract the `{variable}` values from a matching path.
    pub fn capture(&self, path: &str) -> Option<HashMap<String, String>> {
        let captures = self.regex.captures(path)?;
        let mut variables = HashMap::with_capacity(self.variables.len());

        for (index, name) in self.variables.iter().enumerate() {
            if let Some(value) = captures.get(index + 1) {
                variables.insert(name.clone(), value.as_str().to_owned());
            }
        }

        Some(variables)
    }
}

/// The `Path` predicate.
#[derive(Debug, Clone)]
pub struct PathRoutePredicateFactory {
    /// Accepted patterns; any match accepts the route.
    pub paths: Vec<PathPattern>,
}

impl RoutePredicate for PathRoutePredicateFactory {
    fn matches(&self, session: &mut pingora::protocols::http::ServerSession) -> bool {
        // `uri.path()` excludes the query string, unlike `raw_path()`.
        let path = session.req_header().uri.path().to_owned();

        self.paths.iter().any(|pattern| pattern.is_match(&path))
    }
}

/// Compile a path pattern into a [`PathPattern`].
pub(crate) fn build_path_pattern(pattern: &str) -> Option<PathPattern> {
    let normalized = normalize_path_pattern(pattern)?;
    let (regex, variables) = ant_pattern_to_regex(&normalized)?;

    Some(PathPattern {
        pattern: normalized,
        regex,
        variables,
    })
}

fn normalize_path_pattern(pattern: &str) -> Option<String> {
    let pattern = pattern.trim();
    if pattern.is_empty() {
        return None;
    }

    if pattern.starts_with('/') {
        Some(pattern.to_owned())
    } else {
        Some(format!("/{pattern}"))
    }
}

/// Translate an ANT pattern into an anchored regular expression.
fn ant_pattern_to_regex(pattern: &str) -> Option<(Regex, Vec<String>)> {
    let (source, variables) = if let Some(base) = pattern.strip_suffix("/**") {
        let base = if base.is_empty() { "/" } else { base };
        let (base_source, names) = ant_pattern_to_regex_source(base);
        (format!("^(?:{base_source}(?:/.*)?)$"), names)
    } else {
        let (body, names) = ant_pattern_to_regex_source(pattern);
        (format!("^{body}$"), names)
    };

    Regex::new(&source).ok().map(|regex| (regex, variables))
}

/// Translate the literal and wildcard parts of an ANT pattern.
fn ant_pattern_to_regex_source(pattern: &str) -> (String, Vec<String>) {
    let mut regex = String::with_capacity(pattern.len() * 2);
    let mut variables = Vec::new();
    let chars: Vec<char> = pattern.chars().collect();
    let mut index = 0;

    while index < chars.len() {
        match chars[index] {
            '*' if chars.get(index + 1) == Some(&'*') => {
                regex.push_str(".*");
                index += 2;
            }
            '*' => {
                regex.push_str("[^/]*");
                index += 1;
            }
            '?' => {
                regex.push_str("[^/]");
                index += 1;
            }
            '{' => {
                let Some(end) = chars[index..].iter().position(|ch| *ch == '}') else {
                    regex.push_str("\\{");
                    index += 1;
                    continue;
                };
                let end = index + end;
                let mut name: String = chars[index + 1..end].iter().collect();

                if let Some(stripped) = name.strip_prefix('*') {
                    name = stripped.to_owned();
                    regex.push_str("(.*)");
                } else {
                    regex.push_str("([^/]+)");
                }

                variables.push(name);
                index = end + 1;
            }
            ch if matches!(
                ch,
                '.' | '+' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '$' | '|' | '\\'
            ) =>
            {
                regex.push('\\');
                regex.push(ch);
                index += 1;
            }
            ch => {
                regex.push(ch);
                index += 1;
            }
        }
    }

    (regex, variables)
}

#[cfg(test)]
mod tests {
    use super::build_path_pattern;

    #[test]
    fn ant_pattern_supports_trailing_double_star() {
        let pattern = build_path_pattern("/test/**").unwrap();
        assert!(pattern.is_match("/test"));
        assert!(pattern.is_match("/test/child"));
        assert!(!pattern.is_match("/toast/child"));
    }

    #[test]
    fn ant_pattern_supports_single_star() {
        let pattern = build_path_pattern("/api/*/detail").unwrap();
        assert!(pattern.is_match("/api/v1/detail"));
        assert!(!pattern.is_match("/api/v1/extra/detail"));
    }

    #[test]
    fn path_does_not_include_the_query_string() {
        // The predicate receives `uri.path()`, so `/test?a=1` matches `/test/**`.
        let pattern = build_path_pattern("/test/**").unwrap();
        assert!(pattern.is_match("/test"));
    }

    #[test]
    fn path_captures_named_variables() {
        let pattern = build_path_pattern("/api/{version}/users").unwrap();
        assert_eq!(pattern.variables, vec!["version".to_owned()]);

        let variables = pattern.capture("/api/v1/users").unwrap();
        assert_eq!(variables.get("version").map(String::as_str), Some("v1"));
        assert!(pattern.capture("/api/v1/orders").is_none());
    }

    #[test]
    fn star_variable_captures_across_segments() {
        let pattern = build_path_pattern("/files/{*rest}").unwrap();
        let variables = pattern.capture("/files/a/b/c.txt").unwrap();
        assert_eq!(variables.get("rest").map(String::as_str), Some("a/b/c.txt"));
    }
}
