//! Parsing of the filter DSL: `Name=arg1, arg2`.
//!
//! The parser is the gatekeeper of the whole gateway: an unknown filter name or
//! a malformed argument must fail at startup, never degrade into a no-op.
//!
//! Spring Cloud Gateway compatible form is `Name=arg1, arg2`. The pre-existing
//! `Name=k: v` form is still accepted for header filters and logs a deprecation
//! warning.

use crate::config::ConfigError;

/// Which proxy pass a filter belongs to.
///
/// Filters declare their pass instead of probing `exchange.request_header()` at
/// run time; a filter that runs in the wrong pass is how the old rate limiter
/// ended up counting every request twice.
///
/// There is no request *body* phase: the only body limit is `RequestSize`, whose
/// `Content-Length` check runs in the request pass and whose chunked-body check
/// lives in the proxy's `request_body_filter`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterPhase {
    /// Runs once, before the upstream connection, with the request header.
    RequestHeader,
    /// Runs once, with the upstream response header.
    ResponseHeader,
    /// Runs once, with the fully buffered upstream response body.
    ResponseBody,
}

/// A parsed filter expression: the name plus its comma separated arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FilterSpec {
    /// Filter name, exactly as written in the configuration.
    pub name: String,
    /// Arguments after the first `=`, trimmed and comma separated.
    pub args: Vec<String>,
}

impl FilterSpec {
    /// Split `Name=arg1, arg2` into a name and its arguments.
    pub(crate) fn parse(raw: &str) -> Result<Self, ConfigError> {
        let text = raw.trim();
        if text.is_empty() {
            return Err(ConfigError::new("filters", "empty filter expression"));
        }

        let (name, rest) = match text.split_once('=') {
            Some((name, rest)) => (name.trim(), Some(rest)),
            None => (text, None),
        };

        if name.is_empty() {
            return Err(ConfigError::new(
                "filters",
                format!("filter expression `{text}` has no name"),
            ));
        }

        let args = match rest {
            Some(rest) if !rest.trim().is_empty() => rest
                .split(',')
                .map(|arg| arg.trim().to_string())
                .filter(|arg| !arg.is_empty())
                .collect(),
            _ => Vec::new(),
        };

        Ok(Self {
            name: name.to_string(),
            args,
        })
    }
}

/// Check that a filter received between `min` and `max` arguments.
pub(crate) fn expect_args(spec: &FilterSpec, min: usize, max: usize) -> Result<&[String], ConfigError> {
    let count = spec.args.len();
    if count < min || count > max {
        let expected = if min == max {
            format!("{min}")
        } else {
            format!("{min} to {max}")
        };
        return Err(ConfigError::new(
            format!("filters.{}", spec.name),
            format!(
                "`{}` expects {expected} argument(s), got {count}",
                spec.name
            ),
        ));
    }

    Ok(&spec.args)
}

/// Build the error raised for an unknown filter or predicate name.
pub(crate) fn unknown_name_error(kind: &str, name: &str, known: &[&str]) -> ConfigError {
    let suggestion = closest_name(name, known)
        .map(|candidate| format!(", did you mean `{candidate}`?"))
        .unwrap_or_else(|| ". ".to_string());

    ConfigError::new(
        format!("{kind}s.{name}"),
        format!(
            "unknown {kind} `{name}`{suggestion} known {kind}s: {}",
            known.join(", ")
        ),
    )
}

/// Find the candidate with the smallest edit distance, if it is close enough.
fn closest_name<'a>(name: &str, known: &[&'a str]) -> Option<&'a str> {
    let lowered = name.to_ascii_lowercase();
    known
        .iter()
        .map(|candidate| (edit_distance(&lowered, &candidate.to_ascii_lowercase()), *candidate))
        .filter(|(distance, candidate)| {
            *distance <= 2 || candidate.to_ascii_lowercase().starts_with(&lowered)
        })
        .min_by_key(|(distance, _)| *distance)
        .map(|(_, candidate)| candidate)
}

/// Levenshtein distance, used only to build a helpful error message.
fn edit_distance(left: &str, right: &str) -> usize {
    let right_chars: Vec<char> = right.chars().collect();
    let mut previous: Vec<usize> = (0..=right_chars.len()).collect();
    let mut current = vec![0_usize; right_chars.len() + 1];

    for (left_index, left_char) in left.chars().enumerate() {
        current[0] = left_index + 1;
        for (right_index, right_char) in right_chars.iter().enumerate() {
            let substitution = previous[right_index] + usize::from(left_char != *right_char);
            current[right_index + 1] = (previous[right_index + 1] + 1)
                .min(current[right_index] + 1)
                .min(substitution);
        }
        std::mem::swap(&mut previous, &mut current);
    }

    previous[right_chars.len()]
}

#[cfg(test)]
mod tests {
    use super::{expect_args, unknown_name_error, FilterPhase, FilterSpec};
    use crate::filter::gateway_filter::DefaultGatewayFilter;

    #[test]
    fn unknown_filter_name_is_an_error_listing_known_names() {
        let err = DefaultGatewayFilter::try_from("AddRequestHeaderX=Foo, bar").unwrap_err();
        assert!(err.message.contains("AddRequestHeader"), "{}", err.message);
        assert!(err.path.contains("filters"), "{}", err.path);
    }

    #[test]
    fn comma_syntax_is_primary_and_colon_is_legacy() {
        let filter = DefaultGatewayFilter::try_from("AddRequestHeader=X-A, 1, X-B, 2").unwrap();
        assert_eq!(
            filter.header_pairs(),
            &[
                ("X-A".to_string(), "1".to_string()),
                ("X-B".to_string(), "2".to_string())
            ]
        );

        let legacy = DefaultGatewayFilter::try_from("AddRequestHeader=X-A: 1").unwrap();
        assert_eq!(
            legacy.header_pairs(),
            &[("X-A".to_string(), "1".to_string())]
        );
    }

    #[test]
    fn invalid_numeric_argument_is_an_error_not_a_silent_default() {
        let err = DefaultGatewayFilter::try_from("SetStatus=abc").unwrap_err();
        assert!(err.message.contains("100"), "{}", err.message);
    }

    #[test]
    fn spec_splits_name_and_arguments() {
        let spec = FilterSpec::parse("StripPrefix=1").unwrap();
        assert_eq!(spec.name, "StripPrefix");
        assert_eq!(spec.args, vec!["1".to_string()]);
    }

    #[test]
    fn spec_without_arguments_has_an_empty_argument_list() {
        let spec = FilterSpec::parse("SecureHeaders").unwrap();
        assert_eq!(spec.name, "SecureHeaders");
        assert!(spec.args.is_empty());
    }

    #[test]
    fn spec_rejects_an_empty_expression() {
        assert!(FilterSpec::parse("   ").is_err());
    }

    #[test]
    fn expect_args_enforces_the_declared_arity() {
        let spec = FilterSpec::parse("RewritePath=/a, /b").unwrap();
        assert_eq!(expect_args(&spec, 2, 2).unwrap().len(), 2);
        assert!(expect_args(&spec, 3, 3).is_err());
    }

    #[test]
    fn unknown_name_error_lists_candidates() {
        let error = unknown_name_error("filter", "StrpPrefix", &["StripPrefix"]);
        assert!(error.message.contains("StrpPrefix"));
        assert!(error.message.contains("StripPrefix"));
    }

    #[test]
    fn phases_separate_request_and_response_filters() {
        let request = DefaultGatewayFilter::try_from("AddRequestHeader=X-A, 1").unwrap();
        assert_eq!(request.phase(), FilterPhase::RequestHeader);

        let response = DefaultGatewayFilter::try_from("AddResponseHeader=X-B, 1").unwrap();
        assert_eq!(response.phase(), FilterPhase::ResponseHeader);

        let body = DefaultGatewayFilter::try_from("RemoveJsonAttributesResponseBody=a").unwrap();
        assert_eq!(body.phase(), FilterPhase::ResponseBody);

        let size = DefaultGatewayFilter::try_from("RequestSize=1024").unwrap();
        assert_eq!(size.phase(), FilterPhase::RequestHeader);
    }
}
