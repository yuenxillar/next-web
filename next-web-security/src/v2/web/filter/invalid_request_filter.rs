use std::ops::{Deref, DerefMut};

use next_web_core::http::StatusCode;
use next_web_core::{
    Named, Required, async_trait,
    http::{HttpRequest, HttpResponse},
};

use crate::{
    core::util::object::Object,
    web::filter::{
        access_control_filter::{AccessControlFilter, AccessControlFilterExt},
        advice_filter::AdviceFilterExt,
        once_per_request_filter::OncePerRequestFilter,
        path_matching_filter::{PathMatchingFilter, PathMatchingFilterExt},
    },
};

#[derive(Clone)]
pub struct InvalidRequestFilter {
    block_semicolon: bool,
    block_backslash: bool,
    block_non_ascii: bool,
    block_traversal: bool,
    block_encoded_period: bool,
    block_encoded_forward_slash: bool,
    block_rewrite_traversal: bool,

    access_control_filter: AccessControlFilter,
}

impl InvalidRequestFilter {
    const SEMICOLON: &[&str] = &[";", "%3b", "%3B"];
    const BACKSLASH: &[&str] = &["\\", "%5c", "%5C"];
    const FORWARDSLASH: &[&str] = &["%2f", "%2F"];
    const PERIOD: &[&str] = &["%2e", "%2E"];

    fn is_vaild(&self, uri: &str) -> bool {
        uri.is_empty()
            || (!self.contains_semicolon(uri)
                && !self.contains_backslash(uri)
                && !self.contains_non_ascii_characters(uri)
                && !self.contains_traversal(uri)
                && !self.contains_encoded_periods(uri)
                && !self.contains_encoded_forward_slash(uri))
    }

    fn contains_semicolon(&self, uri: &str) -> bool {
        if self.is_block_semicolon() {
            return Self::SEMICOLON.iter().any(|&s| uri.contains(s));
        }

        false
    }

    fn contains_backslash(&self, uri: &str) -> bool {
        if self.is_block_backslash() {
            return Self::BACKSLASH.iter().any(|&s| uri.contains(s));
        }

        false
    }

    fn contains_non_ascii_characters(&self, uri: &str) -> bool {
        if self.is_block_non_ascii() {
            return !self.contains_only_printable_ascii_characters(uri);
        }

        false
    }

    fn contains_only_printable_ascii_characters(&self, uri: &str) -> bool {
        uri.bytes().all(|b| b >= 0x20 && b <= 0x7E)
    }

    fn contains_traversal(&self, uri: &str) -> bool {
        if self.is_block_traversal() {
            return !self.is_normalized(uri)
                || (self.is_block_rewrite_traversal()
                    && ["/..;", "/.;"].iter().any(|s| uri.contains(s)));
        }

        false
    }

    fn contains_encoded_periods(&self, uri: &str) -> bool {
        if self.is_block_encoded_period() {
            return Self::PERIOD.iter().any(|&s| uri.contains(s));
        }

        false
    }

    fn contains_encoded_forward_slash(&self, uri: &str) -> bool {
        if self.is_block_encoded_forward_slash() {
            return Self::FORWARDSLASH.iter().any(|&s| uri.contains(s));
        }

        false
    }

    fn is_normalized(&self, uri: &str) -> bool {
        if uri.is_empty() {
            return true;
        }

        let bytes = uri.as_bytes();
        let mut i = bytes.len();

        while i > 0 {
            // 从右向左查找最后一个 '/'
            let slash_index = bytes[..i].iter().rposition(|&b| b == b'/').unwrap_or(0);

            let gap = i - slash_index;

            // 检查 "." 模式
            if gap == 2 && bytes.get(slash_index + 1) == Some(&b'.') {
                return false;
            }

            // 检查 ".." 模式
            if gap == 3
                && bytes.get(slash_index + 1) == Some(&b'.')
                && bytes.get(slash_index + 2) == Some(&b'.')
            {
                return false;
            }

            i = slash_index;
        }

        true
    }

    fn is_block_rewrite_traversal(&self) -> bool {
        self.block_rewrite_traversal
    }

    pub fn is_block_semicolon(&self) -> bool {
        self.block_semicolon
    }

    pub fn is_block_backslash(&self) -> bool {
        self.block_backslash
    }

    pub fn is_block_non_ascii(&self) -> bool {
        self.block_non_ascii
    }

    pub fn is_block_traversal(&self) -> bool {
        self.block_traversal
    }

    pub fn is_block_encoded_period(&self) -> bool {
        self.block_encoded_period
    }

    pub fn is_block_encoded_forward_slash(&self) -> bool {
        self.block_encoded_forward_slash
    }
}

#[async_trait]
impl AccessControlFilterExt for InvalidRequestFilter {
    async fn is_access_allowed(
        &self,
        request: &mut dyn HttpRequest,
        _response: &mut dyn HttpResponse,
        _mapped_value: Option<Object>,
    ) -> bool {
        // The path the request is served with is the path within the
        // application, which is what Shiro's `getPathWithinApplication` returns
        // and what the traversal checks look at.
        self.is_vaild(request.path())
    }

    async fn on_access_denied(
        &self,
        _request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        _mapped_value: Option<Object>,
    ) -> bool {
        response.set_status_code(StatusCode::BAD_REQUEST);
        response.set_body("Invalid request".into());

        false
    }
}

#[async_trait]
#[async_trait]
impl AdviceFilterExt for InvalidRequestFilter {
    async fn pre_handle(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        ext: Option<&dyn PathMatchingFilterExt>,
    ) -> bool {
        // The validity check is a path-configuration check: it runs for the path
        // the filter was configured for — the path of the chain it serves — and
        // only then is the request checked.
        <Self as Required<PathMatchingFilter>>::get_object(self)
            .pre_handle(request, response, ext)
            .await
    }
}

impl Required<OncePerRequestFilter> for InvalidRequestFilter {
    fn get_object(&self) -> &OncePerRequestFilter {
        &self.access_control_filter.once_per_request_filter
    }

    fn get_mut_object(&mut self) -> &mut OncePerRequestFilter {
        &mut self.access_control_filter.advice_filter
    }
}

impl Required<PathMatchingFilter> for InvalidRequestFilter {
    fn get_object(&self) -> &PathMatchingFilter {
        &self.access_control_filter.path_matching_filter
    }

    fn get_mut_object(&mut self) -> &mut PathMatchingFilter {
        &mut self.access_control_filter.path_matching_filter
    }
}

impl Named for InvalidRequestFilter {
    fn name(&self) -> &str {
        "InvalidRequestFilter"
    }
}

impl Deref for InvalidRequestFilter {
    type Target = AccessControlFilter;

    fn deref(&self) -> &Self::Target {
        &self.access_control_filter
    }
}

impl DerefMut for InvalidRequestFilter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.access_control_filter
    }
}

impl Default for InvalidRequestFilter {
    fn default() -> Self {
        Self {
            block_semicolon: true,
            block_backslash: if cfg!(windows) { false } else { true },
            block_non_ascii: true,
            block_traversal: true,
            block_encoded_period: true,
            block_encoded_forward_slash: true,
            block_rewrite_traversal: true,
            access_control_filter: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::InvalidRequestFilter;

    fn filter() -> InvalidRequestFilter {
        InvalidRequestFilter::default()
    }

    #[test]
    fn a_plain_path_is_valid() {
        assert!(filter().is_vaild("/orders/42"));
        assert!(filter().is_vaild("/"));
        assert!(filter().is_vaild(""));
    }

    #[test]
    fn a_semicolon_in_the_path_is_refused() {
        assert!(!filter().is_vaild("/orders;jsessionid=1"));
        assert!(!filter().is_vaild("/orders%3bjsessionid=1"));
    }

    #[test]
    fn a_backslash_in_the_path_is_refused_where_it_is_not_a_separator() {
        let filter = filter();

        if cfg!(windows) {
            // A backslash separates the segments of a path on Windows, so the
            // default configuration leaves it alone.
            assert!(filter.is_block_backslash() == false);
        } else {
            assert!(!filter.is_vaild("/orders\\42"));
        }
    }

    #[test]
    fn an_encoded_period_and_an_encoded_forward_slash_are_refused() {
        assert!(!filter().is_vaild("/orders/%2e%2e/etc"));
        assert!(!filter().is_vaild("/orders%2f42"));
    }

    #[test]
    fn a_traversal_is_refused() {
        assert!(!filter().is_vaild("/../etc/passwd"));
        assert!(!filter().is_vaild("/orders/../../etc/passwd"));
    }

    #[test]
    fn a_non_ascii_path_is_refused() {
        assert!(!filter().is_vaild("/\u{fc}ber"));
    }
}
