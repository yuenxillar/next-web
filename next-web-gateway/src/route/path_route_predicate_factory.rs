use regex::Regex;

use super::route_predicate::RoutePredicate;

#[derive(Debug, Clone)]
pub struct PathPattern {
    pub pattern: String,
    pub regex: Regex,
}

#[derive(Debug, Clone)]
pub struct PathRoutePredicateFactory {
    pub paths: Vec<PathPattern>,
}

impl RoutePredicate for PathRoutePredicateFactory {
    fn matches(&self, session: &mut pingora::protocols::http::ServerSession) -> bool {
        let path_str = match std::str::from_utf8(session.req_header().raw_path()) {
            Ok(s) => s,
            Err(_) => return false,
        };

        self.paths
            .iter()
            .any(|path_pattern| path_pattern.regex.is_match(path_str))
    }
}

pub(crate) fn build_path_pattern(pattern: &str) -> Option<PathPattern> {
    let normalized = normalize_path_pattern(pattern)?;
    let regex = ant_pattern_to_regex(&normalized)?;
    Some(PathPattern {
        pattern: normalized,
        regex,
    })
}

fn normalize_path_pattern(pattern: &str) -> Option<String> {
    let pattern = pattern.trim();
    if pattern.is_empty() {
        return None;
    }

    if pattern.starts_with('/') {
        Some(pattern.to_string())
    } else {
        Some(format!("/{pattern}"))
    }
}

fn ant_pattern_to_regex(pattern: &str) -> Option<Regex> {
    if let Some(base) = pattern.strip_suffix("/**") {
        let base = if base.is_empty() { "/" } else { base };
        let base_regex = ant_pattern_to_regex_source(base);
        return Regex::new(&format!("^(?:{base_regex}(?:/.*)?)$")).ok();
    }

    Regex::new(&format!("^{}$", ant_pattern_to_regex_source(pattern))).ok()
}

fn ant_pattern_to_regex_source(pattern: &str) -> String {
    let mut regex = String::with_capacity(pattern.len() * 2);
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

    regex
}

#[cfg(test)]
mod tests {
    use super::build_path_pattern;

    #[test]
    fn ant_pattern_supports_trailing_double_star() {
        let pattern = build_path_pattern("/test/**").unwrap();
        assert!(pattern.regex.is_match("/test"));
        assert!(pattern.regex.is_match("/test/child"));
        assert!(!pattern.regex.is_match("/toast/child"));
    }

    #[test]
    fn ant_pattern_supports_single_star() {
        let pattern = build_path_pattern("/api/*/detail").unwrap();
        assert!(pattern.regex.is_match("/api/v1/detail"));
        assert!(!pattern.regex.is_match("/api/v1/extra/detail"));
    }
}
