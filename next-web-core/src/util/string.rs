use std::collections::HashMap;

pub struct StringUtils;

impl StringUtils {
    pub fn has_text(s: &str) -> bool {
        !Self::is_blank(s)
    }

    /// Determine whether the string is null or empty
    pub fn is_empty(s: &str) -> bool {
        s.trim().is_empty()
    }

    /// Determine if the string is not empty
    pub fn is_not_empty(s: &str) -> bool {
        !Self::is_empty(s)
    }

    /// Determine whether the string is blank
    pub fn is_blank(s: &str) -> bool {
        s.chars().all(char::is_whitespace)
    }

    /// Determine whether the string is not blank
    pub fn is_not_blank(s: &str) -> bool {
        !Self::is_blank(s)
    }

    /// Safely retrieve string content, return default value if empty
    pub fn or_default<'a>(s: &'a str, default: &'a str) -> &'a str {
        if Self::is_empty(s) { default } else { s }
    }

    /// Truncate the string to the specified length, and replace the excess with ellipses
    pub fn truncate(s: &str, max_len: usize) -> String {
        if s.chars().count() <= max_len {
            s.to_string()
        } else {
            let truncated: String = s.chars().take(max_len).collect();
            format!("{}...", truncated)
        }
    }

    /// Reverse string (proper handling of Unicode)
    pub fn reverse(s: &str) -> String {
        s.chars().rev().collect()
    }

    /// Remove all blank characters (spaces, line breaks, tabs, etc.)
    pub fn remove_whitespace(s: &str) -> String {
        s.chars().filter(|c| !c.is_whitespace()).collect()
    }

    /// Convert string to camelCase naming
    pub fn to_camel_case(s: &str) -> String {
        let mut result = String::new();
        let mut capitalize_next = false;

        for c in s.chars() {
            if c == '_' || c == '-' || c.is_whitespace() {
                capitalize_next = true;
            } else if capitalize_next {
                result.push(c.to_ascii_uppercase());
                capitalize_next = false;
            } else {
                result.push(c.to_ascii_lowercase());
            }
        }
        result
    }

    /// Convert string to snake named (snake_case)
    pub fn to_snake_case(s: &str) -> String {
        let mut result = String::new();
        let chars: Vec<char> = s.chars().collect();

        for (i, c) in chars.iter().enumerate() {
            if c.is_uppercase() && i > 0 {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
        }
        result
    }

    /// Parse key value pair strings (such as "key1=value1&key2=value2")
    pub fn parse_key_value(s: &str, pair_sep: char, kv_sep: char) -> HashMap<String, String> {
        let mut map = HashMap::new();
        for pair in s.split(pair_sep) {
            if let Some((k, v)) = pair.split_once(kv_sep) {
                map.insert(k.trim().to_string(), v.trim().to_string());
            }
        }
        map
    }

    /// Count the number of occurrences of each character in a string
    pub fn char_frequency(s: &str) -> HashMap<char, usize> {
        let mut freq = HashMap::new();
        for c in s.chars() {
            *freq.entry(c).or_insert(0) += 1;
        }
        freq
    }
}
