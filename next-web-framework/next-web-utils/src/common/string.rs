use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

/// `StringUtil` 是一个工具结构体，用于处理字符串相关的操作。
pub struct StringUtil;

impl StringUtil {
    /// 生成指定长度的随机字母数字字符串 <button class="citation-flag" data-index="1">。
    ///
    /// # 参数
    /// - `length`: 随机字符串的长度。
    ///
    /// # 返回值
    /// 返回一个由 ASCII 字母和数字组成的随机字符串。
    pub fn generate_random_string(length: usize) -> String {
        let rand_string: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(length)
            .map(char::from)
            .collect();
        return rand_string;
    }

    /// 检查字符串是否为空或仅包含空白字符。
    ///
    /// # 参数
    /// - `s`: 待检查的字符串。
    ///
    /// # 返回值
    /// 如果字符串为空或仅包含空白字符，则返回 `true`；否则返回 `false`。
    pub fn is_blank(s: &str) -> bool {
        s.trim_end().is_empty()
    }

    /// 检查字符串是否非空且不全是空白字符。
    ///
    /// # 参数
    /// - `s`: 待检查的字符串。
    ///
    /// # 返回值
    /// 如果字符串非空且不全是空白字符，则返回 `true`；否则返回 `false`。
    pub fn is_not_blank(s: &str) -> bool {
        !StringUtil::is_blank(s)
    }

    /// 检查字符串是否仅包含数字字符。
    ///
    /// # 参数
    /// - `s`: 待检查的字符串。
    ///
    /// # 返回值
    /// 如果字符串仅包含数字字符，则返回 `true`；否则返回 `false`。
    pub fn is_numeric(s: &str) -> bool {
        s.chars().all(|c| c.is_numeric())
    }

    /// 检查字符串是否仅包含字母字符。
    ///
    /// # 参数
    /// - `s`: 待检查的字符串。
    ///
    /// # 返回值
    /// 如果字符串仅包含字母字符，则返回 `true`；否则返回 `false`。
    pub fn is_alpha(s: &str) -> bool {
        s.chars().all(|c| c.is_alphabetic())
    }

    /// 检查字符串是否为纯文本（仅包含字母数字字符或空白字符）。
    ///
    /// # 参数
    /// - `s`: 待检查的字符串。
    ///
    /// # 返回值
    /// 如果字符串为纯文本，则返回 `true`；否则返回 `false`。
    pub fn is_text(s: &str) -> bool {
        !s.trim().is_empty() && s.chars().all(|c| c.is_alphanumeric() || c.is_whitespace())
    }

    /// 对字符串进行 URL 编码。
    ///
    /// # 参数
    /// - `input`: 待编码的字符串。
    ///
    /// # 返回值
    /// 返回经过 URL 编码的字符串。
    pub fn url_encode(input: &str) -> String {
        let mut encoded = String::new();
        for c in input.chars() {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '~' {
                encoded.push(c);
            } else {
                let mut buffer = [0; 4];
                for &byte in c.encode_utf8(&mut buffer).as_bytes() {
                    encoded.push('%');
                    encoded.push_str(&format!("{:02X}", byte));
                }
            }
        }
        encoded
    }

    /// 截取字符串的一部分。
    ///
    /// # 参数
    /// - `s`: 原始字符串。
    /// - `start`: 起始索引（从 0 开始）。
    /// - `end`: 结束索引（不包含该位置）。
    ///
    /// # 返回值
    /// 返回截取后的子字符串。如果索引超出范围，则返回空字符串。
    pub fn substring(s: &str, start: usize, end: usize) -> String {
        s.chars()
            .skip(start)
            .take(end.saturating_sub(start))
            .collect()
    }

    /// 反转字符串。
    ///
    /// # 参数
    /// - `s`: 原始字符串。
    ///
    /// # 返回值
    /// 返回反转后的字符串。
    pub fn reverse(s: &str) -> String {
        s.chars().rev().collect()
    }

    /// 将字符串转换为大写。
    ///
    /// # 参数
    /// - `s`: 原始字符串。
    ///
    /// # 返回值
    /// 返回转换为大写的字符串。
    pub fn to_uppercase(s: &str) -> String {
        s.to_uppercase()
    }

    /// 将字符串转换为小写。
    ///
    /// # 参数
    /// - `s`: 原始字符串。
    ///
    /// # 返回值
    /// 返回转换为小写的字符串。
    pub fn to_lowercase(s: &str) -> String {
        s.to_lowercase()
    }

    /// 去除字符串中的重复字符。
    ///
    /// # 参数
    /// - `s`: 原始字符串。
    ///
    /// # 返回值
    /// 返回去重后的字符串。
    pub fn remove_duplicates(s: &str) -> String {
        let mut seen = std::collections::HashSet::new();
        s.chars().filter(|&c| seen.insert(c)).collect()
    }

    /// 检查字符串是否以指定前缀开头。
    ///
    /// # 参数
    /// - `s`: 原始字符串。
    /// - `prefix`: 待检查的前缀。
    ///
    /// # 返回值
    /// 如果字符串以指定前缀开头，则返回 `true`；否则返回 `false`。
    pub fn starts_with(s: &str, prefix: &str) -> bool {
        s.starts_with(prefix)
    }

    /// 检查字符串是否以指定后缀结尾。
    ///
    /// # 参数
    /// - `s`: 原始字符串。
    /// - `suffix`: 待检查的后缀。
    ///
    /// # 返回值
    /// 如果字符串以指定后缀结尾，则返回 `true`；否则返回 `false`。
    pub fn ends_with(s: &str, suffix: &str) -> bool {
        s.ends_with(suffix)
    }

    /// 检查字符串是否包含空白字符串。
    ///
    /// # 参数
    /// - `s`: 原始字符串。
    ///
    /// # 返回值
    /// 如果字符串包含空白，则返回 `true`；否则返回 `false`。
    pub fn contains_whitespace(s: &str) -> bool {
        s.chars().any(|c| c.is_whitespace())
    }
}
