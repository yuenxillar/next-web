use crate::core::util::pattern_matcher::PatternMatcher;


/// Ant 风格路径匹配器的 Rust 实现
/// 支持 ? * ** 等通配符
#[derive(Debug, Clone)]
pub struct AntPathMatcher {
    path_separator: String,
}

impl Default for AntPathMatcher {
    fn default() -> Self {
        Self {
            path_separator: Self::DEFAULT_PATH_SEPARATOR.to_string(),
        }
    }
}

impl AntPathMatcher {
    const DEFAULT_PATH_SEPARATOR: &str = "/";

    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_path_separator(path_separator: &str) -> Self {
        Self {
            path_separator: path_separator.to_string(),
        }
    }

    /// 设置路径分隔符
    pub fn set_path_separator(&mut self, path_separator: &str) {
        self.path_separator = path_separator.to_string();
    }

    /// 检查路径是否包含模式字符 (* 或 ?)
    pub fn is_pattern(&self, path: &str) -> bool {
        if path.is_empty() {
            return false;
        }
        path.contains('*') || path.contains('?')
    }


    /// 部分匹配模式（用于匹配开始部分）
    pub fn match_start(&self, pattern: &str, path: &str) -> bool {
        self.do_match(pattern, path, false)
    }

    /// 核心匹配算法
    fn do_match(&self, pattern: &str, path: &str, full_match: bool) -> bool {
        // 基础检查
        if path.is_empty()
            || path.starts_with(&self.path_separator) != pattern.starts_with(&self.path_separator)
        {
            return false;
        }

        let patt_dirs = self.tokenize_to_array(pattern);
        let path_dirs = self.tokenize_to_array(path);

        let mut patt_idx_start = 0;
        let mut patt_idx_end = patt_dirs.len().saturating_sub(1);
        let mut path_idx_start = 0;
        let mut path_idx_end = path_dirs.len().saturating_sub(1);

        // 匹配第一个 ** 之前的所有元素
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
                return if pattern.ends_with(&self.path_separator) {
                    path.ends_with(&self.path_separator)
                } else {
                    !path.ends_with(&self.path_separator)
                };
            }

            if !full_match {
                return true;
            }

            if patt_idx_start == patt_idx_end
                && patt_dirs[patt_idx_start] == "*"
                && path.ends_with(&self.path_separator)
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
            // 字符串未耗尽，但模式已耗尽，匹配失败
            return false;
        } else if !full_match && patt_dirs[patt_idx_start] == "**" {
            // 由于模式中的 "**" 部分，路径开始肯定匹配
            return true;
        }

        // 匹配到最后一个 '**'
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
            // 字符串已耗尽
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
                // '**/**' 情况，跳过一个
                patt_idx_start += 1;
                continue;
            }

            // 在 strIdxStart 和 strIdxEnd 之间的字符串中查找 padIdxStart 和 padIdxTmp 之间的模式
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

            if found_idx.is_none() {
                return false;
            }

            patt_idx_start = pat_idx_tmp;
            path_idx_start = found_idx.unwrap() + pat_length;
        }

        // 检查剩余的模式部分是否都是 **
        for i in patt_idx_start..=patt_idx_end {
            if patt_dirs[i] != "**" {
                return false;
            }
        }

        true
    }

    /// 字符串级别匹配，处理 * 和 ? 通配符
    fn match_strings(&self, pattern: &str, target: &str) -> bool {
        let pat_chars: Vec<char> = pattern.chars().collect();
        let str_chars: Vec<char> = target.chars().collect();

        let mut pat_idx_start = 0;
        let mut pat_idx_end = pat_chars.len().saturating_sub(1);
        let mut str_idx_start = 0;
        let mut str_idx_end = str_chars.len().saturating_sub(1);

        let contains_star = pattern.contains('*');

        if !contains_star {
            // 没有 '*'，快速路径
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
            // 模式只包含 '*'，匹配任何内容
            return true;
        }

        // 处理第一个星号之前的字符
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
            // 字符串中的所有字符都已使用
            for i in pat_idx_start..=pat_idx_end {
                if pat_chars[i] != '*' {
                    return false;
                }
            }
            return true;
        }

        // 处理最后一个星号之后的字符
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
            // 字符串中的所有字符都已使用
            for i in pat_idx_start..=pat_idx_end {
                if pat_chars[i] != '*' {
                    return false;
                }
            }
            return true;
        }

        // 处理星号之间的模式
        while pat_idx_start != pat_idx_end && str_idx_start <= str_idx_end {
            let mut pat_idx_tmp = None;
            for i in (pat_idx_start + 1)..=pat_idx_end {
                // 修正：pat_idx_start
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
                // 两个连续的星号，跳过第一个
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

            if found_idx.is_none() {
                return false;
            }

            pat_idx_start = pat_idx_tmp;
            str_idx_start = found_idx.unwrap() + pat_length;
        }

        // 检查剩余的模式字符是否都是星号
        for i in pat_idx_start..=pat_idx_end {
            // 修正：pat_idx_end
            if pat_chars[i] != '*' {
                return false;
            }
        }

        true
    }

    /// 将路径分割为组成部分
    fn tokenize_to_array(&self, path: &str) -> Vec<String> {
        if path.is_empty() {
            return Vec::new();
        }

        path.split(&self.path_separator)
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
                        || (path_segment == 0 && !pattern.starts_with(&self.path_separator))
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


impl PatternMatcher for AntPathMatcher  {
    fn matches(&self, pattern: &str, source: &str) -> bool {
        self.do_match(pattern, source, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_matching() {
        let matcher = AntPathMatcher::new();

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
        let matcher = AntPathMatcher::new();

        assert!(matcher.matches("/api/*/v?/users/**", "/api/products/v1/users/123/profile"));
        assert!(matcher.matches("/**/*.html", "/docs/api/index.html"));
        assert!(matcher.matches("/*/*/*", "/a/b/c"));
    }

    #[test]
    fn test_extract_path() {
        let matcher = AntPathMatcher::new();

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
        let matcher = AntPathMatcher::new();

        assert!(matcher.is_pattern("/test/*"));
        assert!(matcher.is_pattern("/test?"));
        assert!(matcher.is_pattern("/**/test"));
        assert!(!matcher.is_pattern("/test"));
        assert!(!matcher.is_pattern("/"));
    }
}
