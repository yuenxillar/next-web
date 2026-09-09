mod request_matcher_entry;

mod and_request_matcher;
mod ant_path_request_matcher;
mod any_request_matcher;
mod media_type_request_matcher;
mod negated_request_matcher;
mod or_request_matcher;
mod path_pattern_request_matcher;
mod request_header_request_matcher;
mod request_matcher;

pub use and_request_matcher::AndRequestMatcher;
pub use ant_path_request_matcher::AntPathRequestMatcher;
pub use any_request_matcher::AnyRequestMatcher;
pub use media_type_request_matcher::MediaTypeRequestMatcher;
pub use negated_request_matcher::NegatedRequestMatcher;
pub use or_request_matcher::OrRequestMatcher;
pub use path_pattern_request_matcher::{Builder, PathPatternRequestMatcher, DEFAULT_BUILDER};
pub use request_header_request_matcher::RequestHeaderRequestMatcher;
pub use request_matcher::{MatchResult, RequestMatcher};
pub use request_matcher_entry::RequestMatcherEntry;

pub enum MatcherInput {
    Matchers(Vec<std::sync::Arc<dyn RequestMatcher>>),
    MethodWithPatterns(next_web_core::http::HttpMethod, Vec<&'static str>),
    Paths(Vec<&'static str>),
    Method(next_web_core::http::HttpMethod),
}

impl Into<MatcherInput> for Vec<std::sync::Arc<dyn RequestMatcher>> {
    fn into(self) -> MatcherInput {
        MatcherInput::Matchers(self)
    }
}

impl Into<MatcherInput> for (next_web_core::http::HttpMethod, Vec<&'static str>) {
    fn into(self) -> MatcherInput {
        MatcherInput::MethodWithPatterns(self.0, self.1)
    }
}

impl Into<MatcherInput> for Vec<&'static str> {
    fn into(self) -> MatcherInput {
        MatcherInput::Paths(self)
    }
}

impl Into<MatcherInput> for next_web_core::http::HttpMethod {
    fn into(self) -> MatcherInput {
        MatcherInput::Method(self)
    }
}

impl<const N: usize> Into<MatcherInput> for &[&'static str; N] {
    fn into(self) -> MatcherInput {
        MatcherInput::Paths(self.to_vec())
    }
}
