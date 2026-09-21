use std::sync::LazyLock;

use regex::Regex;

/// Regular expression for email desensitization.
///
/// Pattern: captures the first one or two characters of the username and the
/// full domain part.
///
/// Note: This is a simplified pattern intended for desensitization only; it
/// does not guarantee validation of all valid email formats.
pub static EMAIL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([^@]{1,2})[^@]*@(.+)$").expect("EMAIL_RE pattern is valid"));

/// Utility for desensitizing common sensitive information such as phone
/// numbers, email addresses, and ID card numbers.
///
/// All methods return the processed string and are designed to be safe with
/// respect to UTF-8 character boundaries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesensitizedUtil;

impl DesensitizedUtil {
    /// Desensitizes a phone number by keeping the first 3 and last 4 characters
    /// and replacing the middle with `****`.
    ///
    /// Returns the original string if it has fewer than 7 characters.
    pub fn phone(phone: &str) -> String {
        let chars: Vec<char> = phone.chars().collect();
        if chars.len() < 7 {
            return phone.to_string();
        }

        let prefix: String = chars[..3].iter().collect();
        let suffix: String = chars[chars.len() - 4..].iter().collect();
        format!("{}****{}", prefix, suffix)
    }

    /// Desensitizes an email address by keeping the first 1–2 characters of the
    /// username and the full domain.
    ///
    /// If the input matches the standard email pattern, returns
    /// `prefix****@domain`. Otherwise, attempts partial desensitization or
    /// returns a default value.
    pub fn email(email: &str) -> String {
        if let Some(caps) = EMAIL_RE.captures(email) {
            let username_prefix = caps.get(1).unwrap().as_str();
            let domain = caps.get(2).unwrap().as_str();
            format!("{}****@{}", username_prefix, domain)
        } else {
            let chars: Vec<char> = email.chars().collect();
            if chars.len() > 5 {
                let prefix: String = chars[..2].iter().collect();
                format!("{}****", prefix)
            } else {
                "****".to_string()
            }
        }
    }

    /// Desensitizes a password by replacing all characters with `*`, up to a
    /// maximum of 12 asterisks.
    ///
    /// Returns an empty string for empty input.
    pub fn password(password: &str) -> String {
        if password.is_empty() {
            return String::new();
        }
        "*".repeat(password.chars().count().min(12))
    }

    /// Desensitizes an ID card number by keeping the first 4 and last 4
    /// characters and replacing the middle with `********`.
    ///
    /// Returns the original string if it has fewer than 8 characters.
    pub fn id_card(id_card: &str) -> String {
        let chars: Vec<char> = id_card.chars().collect();
        if chars.len() < 8 {
            return id_card.to_string();
        }

        let prefix: String = chars[..4].iter().collect();
        let suffix: String = chars[chars.len() - 4..].iter().collect();
        format!("{}********{}", prefix, suffix)
    }

    /// Desensitizes a bank card number by keeping the first 4 and last 4
    /// characters and replacing the middle with asterisks.
    ///
    /// Returns the original string if it has fewer than 8 characters.
    pub fn bank_card(card_number: &str) -> String {
        let chars: Vec<char> = card_number.chars().collect();
        if chars.len() < 8 {
            return card_number.to_string();
        }

        let prefix: String = chars[..4].iter().collect();
        let suffix: String = chars[chars.len() - 4..].iter().collect();
        let stars = "*".repeat(chars.len() - 8);
        format!("{}{}{}", prefix, stars, suffix)
    }

    /// Desensitizes a name by keeping the first character (surname) and
    /// replacing all subsequent characters with `*`.
    ///
    /// A single-character name returns `surname*`.
    pub fn name(name: &str) -> String {
        let mut chars: Vec<char> = name.chars().collect();
        if chars.is_empty() {
            return String::new();
        }

        for ch in chars.iter_mut().skip(1) {
            *ch = '*';
        }

        if chars.len() == 1 {
            chars.push('*');
        }

        chars.into_iter().collect()
    }

    /// Desensitizes an address by keeping a specified number of prefix and
    /// suffix characters and replacing the middle with asterisks.
    ///
    /// Returns the original string if the total kept length is greater than or
    /// equal to the original length.
    pub fn address(address: &str, keep_prefix: usize, keep_suffix: usize) -> String {
        Self::generic(address, keep_prefix, keep_suffix)
    }

    /// Desensitizes an IPv4 address by keeping the first and last octets and
    /// replacing the middle with `*.*`.
    ///
    /// Returns the original string if it is not a standard IPv4 address.
    pub fn ip(ip: &str) -> String {
        let parts: Vec<&str> = ip.split('.').collect();
        if parts.len() != 4 {
            return ip.to_string();
        }
        format!("{}.*.*.{}", parts[0], parts[3])
    }

    /// Generic string desensitization by keeping a specified number of prefix
    /// and suffix characters and replacing the middle with asterisks.
    ///
    /// Returns the original string if the total kept length is greater than or
    /// equal to the original length.
    pub fn generic(text: &str, keep_prefix: usize, keep_suffix: usize) -> String {
        let chars: Vec<char> = text.chars().collect();
        if chars.len() <= keep_prefix + keep_suffix {
            return text.to_string();
        }

        let prefix: String = chars[..keep_prefix].iter().collect();
        let suffix: String = chars[chars.len() - keep_suffix..].iter().collect();
        let stars = "*".repeat(chars.len() - keep_prefix - keep_suffix);
        format!("{}{}{}", prefix, stars, suffix)
    }

    /// Custom desensitization using a regular expression for matching and
    /// replacement.
    ///
    /// # Errors
    ///
    /// Returns [`DesensitizedError::InvalidPattern`] if `pattern` is not a
    /// valid regular expression.
    pub fn custom_pattern(
        text: &str,
        pattern: &str,
        replacement: &str,
    ) -> Result<String, DesensitizedError> {
        let re =
            Regex::new(pattern).map_err(|e| DesensitizedError::InvalidPattern(e.to_string()))?;
        Ok(re.replace_all(text, replacement).to_string())
    }
}

/// Errors that can occur during desensitization.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DesensitizedError {
    /// The provided regular expression pattern is invalid.
    InvalidPattern(String),
}

impl std::fmt::Display for DesensitizedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DesensitizedError::InvalidPattern(msg) => {
                write!(f, "Invalid regex pattern: {}", msg)
            }
        }
    }
}

impl std::error::Error for DesensitizedError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phone_desensitization() {
        assert_eq!(DesensitizedUtil::phone("13800138000"), "138****8000");
        assert_eq!(DesensitizedUtil::phone("13912345678"), "139****5678");
        assert_eq!(DesensitizedUtil::phone("123"), "123");
    }

    #[test]
    fn test_email_desensitization() {
        assert_eq!(
            DesensitizedUtil::email("test@example.com"),
            "te****@example.com"
        );
        assert_eq!(
            DesensitizedUtil::email("ab@example.com"),
            "ab****@example.com"
        );
        assert_eq!(DesensitizedUtil::email("a@b.com"), "a****@b.com");
    }

    #[test]
    fn test_password_desensitization() {
        assert_eq!(DesensitizedUtil::password("password123"), "***********");
        assert_eq!(DesensitizedUtil::password("short"), "*****");
        assert_eq!(DesensitizedUtil::password(""), "");
    }

    #[test]
    fn test_id_card_desensitization() {
        assert_eq!(
            DesensitizedUtil::id_card("110101199001011234"),
            "1101********1234"
        );
        assert_eq!(DesensitizedUtil::id_card("12345678"), "1234****5678");
    }

    #[test]
    fn test_bank_card_desensitization() {
        assert_eq!(
            DesensitizedUtil::bank_card("6222021234567890123"),
            "6222***********0123"
        );
        assert_eq!(DesensitizedUtil::bank_card("12345678"), "1234**78");
    }

    #[test]
    fn test_name_desensitization() {
        assert_eq!(DesensitizedUtil::name("张三"), "张*");
        assert_eq!(DesensitizedUtil::name("李四"), "李*");
        assert_eq!(DesensitizedUtil::name("王"), "王*");
        assert_eq!(DesensitizedUtil::name("诸葛孔明"), "诸***");
    }

    #[test]
    fn test_custom_pattern() {
        let result = DesensitizedUtil::custom_pattern(
            "信用卡号：6222 1234 5678 9012，有效期：12/25",
            r"\d{4} \d{4} \d{4} \d{4}",
            "**** **** **** ****",
        )
        .unwrap();
        assert_eq!(result, "信用卡号：**** **** **** ****，有效期：12/25");
    }
}
