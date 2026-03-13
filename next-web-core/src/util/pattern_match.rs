/// Utility class for simple pattern matching.
///
/// Pattern matching supports '*' wildcards.
pub struct PatternMatchUtils;

impl PatternMatchUtils {
    /// Match a String against the given pattern, supporting '*' wildcards.
    ///
    /// # Arguments
    /// * `pattern` - The pattern to match against. May be `None`.
    /// * `str` - The String to match. May be `None`.
    ///
    /// # Returns
    /// `true` if the String matches the given pattern, `false` otherwise.
    pub fn simple_match(pattern: &str, str: &str) -> bool {
        let first_index = pattern.find('*');

        match first_index {
            None => {
                // No wildcard - exact match required
                pattern == str
            }
            Some(0) => {
                // Pattern starts with '*'
                if pattern.len() == 1 {
                    // Single '*' matches everything
                    return true;
                }

                let next_index = pattern[1..].find('*').map(|i| i + 1);

                match next_index {
                    None => {
                        // No more wildcards - check if str ends with the part after '*'
                        str.ends_with(&pattern[1..])
                    }
                    Some(next) => {
                        // Pattern has another '*'
                        let part = &pattern[1..next];
                        if part.is_empty() {
                            // Consecutive '**' - skip and continue
                            Self::simple_match(&pattern[next..], str)
                        } else {
                            // Find first occurrence of 'part' in str
                            let mut part_index = 0;
                            while let Some(idx) = str[part_index..].find(part) {
                                let actual_idx = part_index + idx;
                                if Self::simple_match(
                                    &pattern[next..],
                                    &str[actual_idx + part.len()..],
                                ) {
                                    return true;
                                }
                                part_index = actual_idx + 1;
                                if part_index >= str.len() {
                                    break;
                                }
                            }
                            false
                        }
                    }
                }
            }
            Some(first) => {
                // Pattern starts with text before first '*'
                str.len() >= first
                    && pattern.starts_with(&str[..first])
                    && Self::simple_match(&pattern[first..], &str[first..])
            }
        }
    }

    /// Match a String against multiple patterns, supporting '*' wildcards.
    ///
    /// # Arguments
    /// * `patterns` - The patterns to match against. May be `None`.
    /// * `str` - The String to match.
    ///
    /// # Returns
    /// `true` if the String matches any of the given patterns, `false` otherwise.
    pub fn simple_match_any(patterns: Option<&[&str]>, str: &str) -> bool {
        if let Some(patterns) = patterns {
            for pattern in patterns {
                if Self::simple_match(pattern, str) {
                    return true;
                }
            }
        }
        false
    }
}
