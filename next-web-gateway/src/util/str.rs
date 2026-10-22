//! Small string helpers shared by the predicate implementations.

/// Utilities used by the route predicates.
pub struct StrUtil;

impl StrUtil {
    /// Match a host against an ANT style pattern.
    ///
    /// `*` matches exactly one label, `**` matches zero or more labels, and a
    /// literal label must be equal (case insensitive, port and trailing dot
    /// ignored).
    pub fn host_match(host: &str, pattern: &str) -> bool {
        let host = host_labels(host);
        let pattern = host_labels(pattern);

        labels_match(&host, &pattern)
    }
}

/// Lowercase and split a host into labels, dropping the port and empty labels.
fn host_labels(host: &str) -> Vec<String> {
    let host = host.trim().trim_end_matches('.').to_ascii_lowercase();

    let host = if let Some(stripped) = host.strip_prefix('[') {
        match stripped.split_once(']') {
            Some((ipv6, _)) => ipv6.to_string(),
            None => host.clone(),
        }
    } else if host.matches(':').count() == 1 {
        host.split_once(':')
            .map(|(name, _)| name.to_string())
            .unwrap_or(host.clone())
    } else {
        host
    };

    host.split('.')
        .filter(|label| !label.is_empty())
        .map(str::to_string)
        .collect()
}

/// Glob match two label sequences, where `**` swallows zero or more labels.
fn labels_match(host: &[String], pattern: &[String]) -> bool {
    let Some((first, rest)) = pattern.split_first() else {
        return host.is_empty();
    };

    if first == "**" {
        // `**` matches zero or more labels, so try every split point.
        return (0..=host.len()).any(|skip| labels_match(&host[skip..], rest));
    }

    let Some((host_first, host_rest)) = host.split_first() else {
        return false;
    };

    if first != "*" && first != host_first {
        return false;
    }

    labels_match(host_rest, rest)
}

#[cfg(test)]
mod tests {
    use super::StrUtil;

    #[test]
    fn exact_host_matches() {
        assert!(StrUtil::host_match("example.com", "example.com"));
        assert!(!StrUtil::host_match("a.example.com", "example.com"));
    }

    #[test]
    fn single_star_matches_exactly_one_label() {
        assert!(StrUtil::host_match("a.example.com", "*.example.com"));
        assert!(!StrUtil::host_match("a.b.example.com", "*.example.com"));
        assert!(!StrUtil::host_match("example.com", "*.example.com"));
    }

    #[test]
    fn double_star_matches_zero_or_more_labels() {
        assert!(StrUtil::host_match("example.com", "**.example.com"));
        assert!(StrUtil::host_match("a.example.com", "**.example.com"));
        assert!(StrUtil::host_match("a.b.example.com", "**.example.com"));
        assert!(!StrUtil::host_match("example.org", "**.example.com"));
    }

    #[test]
    fn port_and_case_are_ignored() {
        assert!(StrUtil::host_match("Example.COM:8080", "example.com"));
        assert!(StrUtil::host_match("example.com.", "example.com"));
    }
}
