use std::borrow::Cow;

/// Ant-style path pattern matcher.
///
/// Matches paths against patterns containing wildcards:
///
/// - `?` matches exactly one character within a path segment.
/// - `*` matches any number of characters within a single path segment.
/// - `**` matches any number of path segments, including none.
///
/// Leading path separators are ignored, so both `*.properties` and
/// `/*.properties` match the relative path `index.properties`, and
/// `**/*.properties` also matches nested paths such as
/// `messages/zh-CN.properties`. Trailing separators, by contrast, must
/// match explicitly.
///
/// # Examples
///
/// ```
/// use next_web_core::util::matcher::AntPathMatcher;
///
/// let matcher = AntPathMatcher::new();
/// assert!(matcher.matches("messages/*.properties", "messages/zh-CN.properties"));
/// assert!(matcher.matches("**/*.properties", "messages/zh-CN.properties"));
/// assert!(!matcher.matches("*.properties", "messages/zh-CN.properties"));
/// ```
#[derive(Debug, Clone)]
pub struct AntPathMatcher {
    path_separator: Cow<'static, str>,
}

impl AntPathMatcher {
    const DEFAULT_PATH_SEPARATOR: &str = "/";

    /// Create a new instance of AntPathMatcher using the specified path delimiter.
    pub fn new<S>(path_separator: S) -> Self
    where
        S: Into<Cow<'static, str>>,
    {
        Self {
            path_separator: path_separator.into(),
        }
    }

    /// Match the given path against the pattern, returning `true` when the
    /// whole path matches.
    pub fn matches(&self, pattern: &str, source: &str) -> bool {
        self.do_match(pattern, source, true)
    }

    /// Set path delimiter
    pub fn set_path_separator<S>(&mut self, path_separator: S)
    where
        S: Into<Cow<'static, str>>,
    {
        self.path_separator = path_separator.into();
    }

    /// Check if the path contains pattern characters (* or ?)
    pub fn is_pattern(&self, path: &str) -> bool {
        if path.is_empty() {
            return false;
        }
        path.contains('*') || path.contains('?')
    }

    /// Partial match pattern (used to match the start of the path)
    pub fn match_start(&self, pattern: &str, path: &str) -> bool {
        self.do_match(pattern, path, false)
    }

    /// Core matching algorithm.
    ///
    /// When `full_match` is `false`, only the beginning of the path has to
    /// match the pattern; this is used by [`match_start`](Self::match_start).
    fn do_match(&self, pattern: &str, path: &str, full_match: bool) -> bool {
        let patt_dirs = self.tokenize_to_array(pattern);
        let path_dirs = self.tokenize_to_array(path);

        // A path without any segment matches only a pattern without any
        // segment. Leading separators have no effect on the tokenized
        // segments, so `/a` and `a` are treated alike.
        if path_dirs.is_empty() {
            return patt_dirs.is_empty();
        }
        if patt_dirs.is_empty() {
            return false;
        }

        let mut patt_idx_start = 0;
        let mut patt_idx_end = patt_dirs.len().saturating_sub(1);
        let mut path_idx_start = 0;
        let mut path_idx_end = path_dirs.len().saturating_sub(1);

        // Match all elements before the first * *
        while patt_idx_start <= patt_idx_end && path_idx_start <= path_idx_end {
            let pat_dir = &patt_dirs[patt_idx_start];
            if pat_dir == "**" {
                break;
            }
            if !self.match_strings(pat_dir, &path_dirs[path_idx_start]) {
                return false;
            }
            patt_idx_start += 1;
            path_idx_start += 1;
        }

        if path_idx_start > path_idx_end {
            // 路径已耗尽，只有当模式的剩余部分是 * 或 ** 时才匹配
            if patt_idx_start > patt_idx_end {
                return if pattern.ends_with(&self.path_separator.as_ref()) {
                    path.ends_with(&self.path_separator.as_ref())
                } else {
                    !path.ends_with(self.path_separator.as_ref())
                };
            }

            if !full_match {
                return true;
            }

            if patt_idx_start == patt_idx_end
                && patt_dirs[patt_idx_start] == "*"
                && path.ends_with(self.path_separator.as_ref())
            {
                return true;
            }

            for i in patt_idx_start..=patt_idx_end {
                if patt_dirs[i] != "**" {
                    return false;
                }
            }
            return true;
        } else if patt_idx_start > patt_idx_end {
            // String not exhausted, but pattern exhausted, matching failed
            return false;
        } else if !full_match && patt_dirs[patt_idx_start] == "**" {
            // Due to the "* *" part in the pattern, the path will definitely match at the beginning
            return true;
        }

        // Match to the last '* *'
        while patt_idx_start <= patt_idx_end && path_idx_start <= path_idx_end {
            let pat_dir = &patt_dirs[patt_idx_end];
            if pat_dir == "**" {
                break;
            }
            if !self.match_strings(pat_dir, &path_dirs[path_idx_end]) {
                return false;
            }
            patt_idx_end = patt_idx_end.saturating_sub(1);
            path_idx_end = path_idx_end.saturating_sub(1);
        }

        if path_idx_start > path_idx_end {
            // String exhausted,
            for i in patt_idx_start..=patt_idx_end {
                if patt_dirs[i] != "**" {
                    return false;
                }
            }
            return true;
        }

        while patt_idx_start != patt_idx_end && path_idx_start <= path_idx_end {
            let mut pat_idx_tmp = None;
            for i in (patt_idx_start + 1)..=patt_idx_end {
                if patt_dirs[i] == "**" {
                    pat_idx_tmp = Some(i);
                    break;
                }
            }

            let pat_idx_tmp = match pat_idx_tmp {
                Some(idx) => idx,
                None => break,
            };

            if pat_idx_tmp == patt_idx_start + 1 {
                patt_idx_start += 1;
                continue;
            }

            // Find the pattern between padIdxStart and padIdxTmp in the string between strIdxStart and strIdxEnd
            let pat_length = pat_idx_tmp - patt_idx_start - 1;
            let str_length = path_idx_end - path_idx_start + 1;
            let mut found_idx = None;

            'str_loop: for i in 0..=(str_length.saturating_sub(pat_length)) {
                for j in 0..pat_length {
                    let sub_pat = &patt_dirs[patt_idx_start + j + 1];
                    let sub_str = &path_dirs[path_idx_start + i + j];
                    if !self.match_strings(sub_pat, sub_str) {
                        continue 'str_loop;
                    }
                }
                found_idx = Some(path_idx_start + i);
                break;
            }

            let Some(found_idx) = found_idx else {
                return false;
            };

            patt_idx_start = pat_idx_tmp;
            path_idx_start = found_idx + pat_length;
        }

        // Check if the remaining mode sections are all**
        for i in patt_idx_start..=patt_idx_end {
            if patt_dirs[i] != "**" {
                return false;
            }
        }

        true
    }

    /// Match a single path segment against a pattern segment, handling the
    /// `*` and `?` wildcards.
    fn match_strings(&self, pattern: &str, target: &str) -> bool {
        let pat_chars: Vec<char> = pattern.chars().collect();
        let str_chars: Vec<char> = target.chars().collect();

        let mut pat_idx_start = 0;
        let mut pat_idx_end = pat_chars.len().saturating_sub(1);
        let mut str_idx_start = 0;
        let mut str_idx_end = str_chars.len().saturating_sub(1);

        let contains_star = pattern.contains('*');

        if !contains_star {
            // No '*', fast path
            if pat_idx_end != str_idx_end {
                return false;
            }
            for i in 0..=pat_idx_end {
                let ch = pat_chars[i];
                if ch != '?' && ch != str_chars[i] {
                    return false;
                }
            }
            return true;
        }

        if pat_idx_end == 0 {
            // The pattern only contains' * ', matching any content
            return true;
        }

        // Process characters before the first asterisk
        while pat_idx_start <= pat_idx_end
            && str_idx_start <= str_idx_end
            && pat_chars[pat_idx_start] != '*'
        {
            let ch = pat_chars[pat_idx_start];
            if ch != '?' && ch != str_chars[str_idx_start] {
                return false;
            }
            pat_idx_start += 1;
            str_idx_start += 1;
        }

        if str_idx_start > str_idx_end {
            // All characters in the string have been used
            for i in pat_idx_start..=pat_idx_end {
                if pat_chars[i] != '*' {
                    return false;
                }
            }
            return true;
        }

        // Process the characters after the last asterisk
        while pat_idx_start <= pat_idx_end
            && str_idx_start <= str_idx_end
            && pat_chars[pat_idx_end] != '*'
        {
            let ch = pat_chars[pat_idx_end];
            if ch != '?' && ch != str_chars[str_idx_end] {
                return false;
            }
            pat_idx_end = pat_idx_end.saturating_sub(1);
            str_idx_end = str_idx_end.saturating_sub(1);
        }

        if str_idx_start > str_idx_end {
            for i in pat_idx_start..=pat_idx_end {
                if pat_chars[i] != '*' {
                    return false;
                }
            }
            return true;
        }

        while pat_idx_start != pat_idx_end && str_idx_start <= str_idx_end {
            let mut pat_idx_tmp = None;
            for i in (pat_idx_start + 1)..=pat_idx_end {
                if pat_chars[i] == '*' {
                    pat_idx_tmp = Some(i);
                    break;
                }
            }

            let pat_idx_tmp = match pat_idx_tmp {
                Some(idx) => idx,
                None => break,
            };

            if pat_idx_tmp == pat_idx_start + 1 {
                pat_idx_start += 1;
                continue;
            }

            let pat_length = pat_idx_tmp - pat_idx_start - 1;
            let str_length = str_idx_end - str_idx_start + 1;
            let mut found_idx = None;

            'inner_loop: for i in 0..=(str_length.saturating_sub(pat_length)) {
                for j in 0..pat_length {
                    let ch = pat_chars[pat_idx_start + j + 1];
                    if ch != '?' && ch != str_chars[str_idx_start + i + j] {
                        continue 'inner_loop;
                    }
                }
                found_idx = Some(str_idx_start + i);
                break;
            }

            let Some(found_idx) = found_idx else {
                return false;
            };

            pat_idx_start = pat_idx_tmp;
            str_idx_start = found_idx + pat_length;
        }

        // Check if all remaining pattern characters are asterisks
        for i in pat_idx_start..=pat_idx_end {
            if pat_chars[i] != '*' {
                return false;
            }
        }

        true
    }

    /// Split the path into components
    fn tokenize_to_array(&self, path: &str) -> Vec<String> {
        if path.is_empty() {
            return Vec::new();
        }

        path.split(self.path_separator.as_ref())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect()
    }

    /// 提取模式匹配的部分路径
    pub fn extract_path_within_pattern(&self, pattern: &str, path: &str) -> String {
        let pattern_parts = self.tokenize_to_array(pattern);
        let path_parts = self.tokenize_to_array(path);
        let mut builder = String::new();
        let mut path_started = false;

        for segment in 0..pattern_parts.len() {
            let pattern_part = &pattern_parts[segment];
            if pattern_part.contains('*') || pattern_part.contains('?') {
                for path_segment in segment..path_parts.len() {
                    if path_started
                        || (path_segment == 0 && !pattern.starts_with(self.path_separator.as_ref()))
                    {
                        builder.push_str(&self.path_separator);
                    }
                    builder.push_str(&path_parts[path_segment]);
                    path_started = true;
                }
                break;
            }
        }

        builder
    }
}

impl Default for AntPathMatcher {
    fn default() -> Self {
        Self {
            path_separator: Cow::Borrowed(Self::DEFAULT_PATH_SEPARATOR),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_matching() {
        let matcher = AntPathMatcher::default();

        // 基本匹配
        assert!(matcher.matches("/test", "/test"));
        assert!(!matcher.matches("/test", "/test2"));

        // 单字符通配符
        assert!(matcher.matches("/t?st", "/test"));
        assert!(matcher.matches("/t?st", "/tast"));
        assert!(!matcher.matches("/t?st", "/test2"));

        // 单段通配符
        assert!(matcher.matches("/*.jpg", "/test.jpg"));
        assert!(matcher.matches("/*.jpg", "/photo.jpg"));
        assert!(!matcher.matches("/*.jpg", "/test.png"));

        // 多段通配符
        assert!(matcher.matches("/**/test", "/api/v1/test"));
        assert!(matcher.matches("/**/test", "/test"));
        assert!(matcher.matches("/resources/**", "/resources/images/photo.jpg"));
    }

    #[test]
    fn test_complex_patterns() {
        let matcher = AntPathMatcher::default();

        assert!(matcher.matches("/api/*/v?/users/**", "/api/products/v1/users/123/profile"));
        assert!(matcher.matches("/**/*.html", "/docs/api/index.html"));
        assert!(matcher.matches("/*/*/*", "/a/b/c"));
    }

    #[test]
    fn test_extract_path() {
        let matcher = AntPathMatcher::default();

        assert_eq!(
            matcher.extract_path_within_pattern("/docs/**", "/docs/cvs/commit"),
            "cvs/commit"
        );
        assert_eq!(
            matcher.extract_path_within_pattern("/docs/*", "/docs/cvs"),
            "cvs"
        );
    }

    #[test]
    fn test_is_pattern() {
        let matcher = AntPathMatcher::default();

        assert!(matcher.is_pattern("/test/*"));
        assert!(matcher.is_pattern("/test?"));
        assert!(matcher.is_pattern("/**/test"));
        assert!(!matcher.is_pattern("/test"));
        assert!(!matcher.is_pattern("/"));
    }

    #[test]
    fn test_relative_paths() {
        let matcher = AntPathMatcher::default();

        // Patterns and paths without a leading separator.
        assert!(matcher.matches("*.jpg", "test.jpg"));
        assert!(matcher.matches("messages/*.properties", "messages/zh-CN.properties"));
        assert!(matcher.matches("**/*.properties", "messages/sub/zh-CN.properties"));
        assert!(matcher.matches("**", "index.html"));
        assert!(matcher.matches("a/*/c", "a/b/c"));
        assert!(matcher.matches("a/**/c", "a/c"));
        assert!(!matcher.matches("*.jpg", "images/test.jpg"));
        assert!(!matcher.matches("a/**/c", "a/b/d"));

        // Leading separators are ignored, so absolute patterns also match
        // relative paths and vice versa.
        assert!(matcher.matches("/*.jpg", "test.jpg"));
        assert!(matcher.matches("/messages/*.properties", "messages/zh-CN.properties"));
        assert!(matcher.matches("/**/test", "api/v1/test"));
        assert!(matcher.matches("*.jpg", "/test.jpg"));

        // Empty inputs.
        assert!(matcher.matches("", ""));
        assert!(!matcher.matches("", "test.jpg"));
        assert!(!matcher.matches("*", ""));
    }
}
