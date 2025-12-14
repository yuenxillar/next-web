use once_cell::sync::Lazy;
use regex::Regex;

/// 邮箱正则表达式，用于邮箱脱敏。
/// 匹配规则：捕获邮箱用户名的前两个字符（至少一个）和完整的域名部分。
/// 注意：这是一个简化模式，用于脱敏目的，不保证验证所有合法邮箱格式。
///
/// Regular expression for email desensitization.
/// Pattern: Captures the first two characters (at least one) of the username and the full domain part.
/// Note: This is a simplified pattern for desensitization purposes and does not guarantee validation of all valid email formats.
pub static EMAIL_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^([^@]{1,2})[^@]*@(.+)$").unwrap());

/// 脱敏工具类
/// 提供一系列静态方法，用于对常见的敏感信息（如手机号、邮箱、身份证号等）进行脱敏处理，
/// 以保护用户隐私。所有方法均返回处理后的字符串。
///
/// Desensitization Utility Class
/// Provides a series of static methods for desensitizing common sensitive information
/// (such as phone numbers, email addresses, ID cards, etc.) to protect user privacy.
/// All methods return the processed string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesensitizedUtil;

impl DesensitizedUtil {
    /// 手机号脱敏 (保留前3后4位，中间用 `****` 代替)
    ///
    /// # 参数
    /// - `phone`: 需要脱敏的手机号字符串
    ///
    /// # 返回值
    /// 脱敏后的手机号字符串。如果输入长度小于7，则返回原字符串。
    ///
    /// Desensitizes a phone number by keeping the first 3 and last 4 digits, replacing the middle with `****`.
    ///
    /// # Parameters
    /// - `phone`: The phone number string to be desensitized.
    ///
    /// # Returns
    /// The desensitized phone number string. Returns the original string if the input length is less than 7.
    pub fn phone(phone: &str) -> String {
        if phone.len() < 7 {
            return phone.to_string();
        }

        let prefix = &phone[..3];
        let suffix = &phone[phone.len() - 4..];
        format!("{}****{}", prefix, suffix)
    }

    /// 邮箱脱敏 (保留用户名前2个字符和完整域名)
    ///
    /// # 参数
    /// - `email`: 需要脱敏的邮箱地址字符串
    ///
    /// # 返回值
    /// 脱敏后的邮箱字符串。如果符合标准邮箱格式，返回前缀+`****@`+域名；否则尝试进行部分脱敏或返回默认值。
    ///
    /// Desensitizes an email address by keeping the first 2 characters of the username and the full domain.
    ///
    /// # Parameters
    /// - `email`: The email address string to be desensitized.
    ///
    /// # Returns
    /// The desensitized email string. If it matches the standard format, returns prefix+`****@`+domain;
    /// otherwise attempts partial desensitization or returns a default value.
    pub fn email(email: &str) -> String {
        if let Some(caps) = EMAIL_RE.captures(email) {
            let username_prefix = caps.get(1).unwrap().as_str();
            let domain = caps.get(2).unwrap().as_str();
            format!("{}****@{}", username_prefix, domain)
        } else {
            // 如果不是标准邮箱格式，返回原始值或部分脱敏
            if email.len() > 5 {
                let prefix = &email[..2];
                format!("{}****", prefix)
            } else {
                "****".to_string()
            }
        }
    }

    /// 密码脱敏 (用 `*` 号代替所有字符，最多显示12个)
    ///
    /// # 参数
    /// - `password`: 需要脱敏的密码字符串
    ///
    /// # 返回值
    /// 由 `*` 号组成的字符串，长度为原密码长度（最多12个）。空字符串返回空。
    ///
    /// Desensitizes a password by replacing all characters with `*`, up to a maximum of 12 asterisks.
    ///
    /// # Parameters
    /// - `password`: The password string to be desensitized.
    ///
    /// # Returns
    /// A string of `*` characters, with length equal to the original password length (capped at 12). Returns an empty string for empty input.
    pub fn password(password: &str) -> String {
        if password.is_empty() {
            return String::default();
        }
        "*".repeat(password.len().min(12)) // 最多显示12个*
    }

    /// 身份证号脱敏 (保留前4后4位，中间用 `********` 代替)
    ///
    /// # 参数
    /// - `id_card`: 需要脱敏的身份证号字符串
    ///
    /// # 返回值
    /// 脱敏后的身份证号字符串。如果输入长度小于8，则返回原字符串。
    ///
    /// Desensitizes an ID card number by keeping the first 4 and last 4 digits, replacing the middle with `********`.
    ///
    /// # Parameters
    /// - `id_card`: The ID card number string to be desensitized.
    ///
    /// # Returns
    /// The desensitized ID card number string. Returns the original string if the input length is less than 8.
    pub fn id_card(id_card: &str) -> String {
        if id_card.len() < 8 {
            return id_card.to_string();
        }

        let prefix = &id_card[..4];
        let suffix = &id_card[id_card.len() - 4..];
        format!("{}********{}", prefix, suffix)
    }

    /// 银行卡号脱敏 (保留前4后4位，中间用 `*` 号代替)
    ///
    /// # 参数
    /// - `card_number`: 需要脱敏的银行卡号字符串
    ///
    /// # 返回值
    /// 脱敏后的银行卡号字符串。如果输入长度小于8，则返回原字符串。
    ///
    /// Desensitizes a bank card number by keeping the first 4 and last 4 digits, replacing the middle with asterisks.
    ///
    /// # Parameters
    /// - `card_number`: The bank card number string to be desensitized.
    ///
    /// # Returns
    /// The desensitized bank card number string. Returns the original string if the input length is less than 8.
    pub fn bank_card(card_number: &str) -> String {
        if card_number.len() < 8 {
            return card_number.to_string();
        }

        let prefix = &card_number[..4];
        let suffix = &card_number[card_number.len() - 4..];
        let stars = "*".repeat(card_number.len() - 8);
        format!("{}{}{}", prefix, stars, suffix)
    }

    /// 姓名脱敏 (保留姓氏的第一个汉字，其余汉字用 `*` 代替)
    ///
    /// # 参数
    /// - `name`: 需要脱敏的姓名字符串
    ///
    /// # 返回值
    /// 脱敏后的姓名字符串。单字姓名返回 `*`。
    ///
    /// Desensitizes a name by keeping the first character (surname) and replacing all subsequent characters with `*`.
    ///
    /// # Parameters
    /// - `name`: The name string to be desensitized.
    ///
    /// # Returns
    /// The desensitized name string. A single-character name returns `*`.
    pub fn name(name: &str) -> String {
        if name.is_empty() {
            return String::new();
        }

        if name.chars().count() == 1 {
            return format!("{}*", name);
        }

        let mut chars: Vec<char> = name.chars().collect();
        for i in 1..chars.len() {
            chars[i] = '*';
        }
        chars.into_iter().collect()
    }

    /// 地址脱敏 (保留指定长度的前缀和后缀，中间用 `*` 号代替)
    ///
    /// # 参数
    /// - `address`: 需要脱敏的地址字符串
    /// - `keep_prefix`: 需要保留的前缀字符数量
    /// - `keep_suffix`: 需要保留的后缀字符数量
    ///
    /// # 返回值
    /// 脱敏后的地址字符串。如果总保留长度大于等于原字符串长度，则返回原字符串。
    ///
    /// Desensitizes an address by keeping a specified number of prefix and suffix characters, replacing the middle with asterisks.
    ///
    /// # Parameters
    /// - `address`: The address string to be desensitized.
    /// - `keep_prefix`: The number of prefix characters to keep.
    /// - `keep_suffix`: The number of suffix characters to keep.
    ///
    /// # Returns
    /// The desensitized address string. Returns the original string if the total kept length is greater than or equal to the original length.
    pub fn address(address: &str, keep_prefix: usize, keep_suffix: usize) -> String {
        if address.len() <= keep_prefix + keep_suffix {
            return address.to_string();
        }

        let prefix = &address[..keep_prefix];
        let suffix = &address[address.len() - keep_suffix..];
        let stars = "*".repeat(address.len() - keep_prefix - keep_suffix);
        format!("{}{}{}", prefix, stars, suffix)
    }

    pub fn ip(ip: &str) -> String {
        let parts: Vec<&str> = ip.split('.').collect();

        // 确保是有效的IPv4地址（4个部分）
        if parts.len() != 4 {
            return ip.to_string(); // 如果不是标准IPv4格式，直接返回原字符串
        }
    
        format!("{}.*.*.{}", parts[0], parts[3])
    }

    /// 通用字符串脱敏 (保留指定长度的前缀和后缀，中间用 `*` 号代替)
    ///
    /// # 参数
    /// - `text`: 需要脱敏的任意字符串
    /// - `keep_prefix`: 需要保留的前缀字符数量
    /// - `keep_suffix`: 需要保留的后缀字符数量
    ///
    /// # 返回值
    /// 脱敏后的字符串。如果总保留长度大于等于原字符串长度，则返回原字符串。
    ///
    /// Generic string desensitization by keeping a specified number of prefix and suffix characters, replacing the middle with asterisks.
    ///
    /// # Parameters
    /// - `text`: The arbitrary string to be desensitized.
    /// - `keep_prefix`: The number of prefix characters to keep.
    /// - `keep_suffix`: The number of suffix characters to keep.
    ///
    /// # Returns
    /// The desensitized string. Returns the original string if the total kept length is greater than or equal to the original length.
    pub fn generic(text: &str, keep_prefix: usize, keep_suffix: usize) -> String {
        if text.len() <= keep_prefix + keep_suffix {
            return text.to_string();
        }

        let prefix = &text[..keep_prefix];
        let suffix = &text[text.len() - keep_suffix..];
        let stars = "*".repeat(text.len() - keep_prefix - keep_suffix);
        format!("{}{}{}", prefix, stars, suffix)
    }

    /// 自定义脱敏模式 (使用正则表达式进行模式匹配和替换)
    ///
    /// # 参数
    /// - `text`: 需要脱敏的原始字符串
    /// - `pattern`: 用于匹配需要脱敏部分的正则表达式模式
    /// - `replacement`: 用于替换匹配部分的字符串（可包含捕获组引用，如 `$1`）
    ///
    /// # 返回值
    /// 经过正则替换后的字符串。如果正则表达式无效，则返回原始字符串。
    ///
    /// Custom desensitization pattern using regular expression matching and replacement.
    ///
    /// # Parameters
    /// - `text`: The original string to be desensitized.
    /// - `pattern`: The regular expression pattern to match the parts to be desensitized.
    /// - `replacement`: The string used to replace the matched parts (can include capture group references like `$1`).
    ///
    /// # Returns
    /// The string after regex replacement. Returns the original string if the regex pattern is invalid.
    pub fn custom_pattern(text: &str, pattern: &str, replacement: &str) -> String {
        let re = Regex::new(pattern).unwrap_or_else(|_| {
            // 如果正则表达式无效，返回原始文本
            Regex::new("^$").unwrap() // 匹配空字符串，不会替换任何内容
        });

        re.replace_all(text, replacement).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phone_desensitization() {
        assert_eq!(DesensitizedUtil::phone("13800138000"), "138****8000");
        assert_eq!(DesensitizedUtil::phone("13912345678"), "139****5678");
        assert_eq!(DesensitizedUtil::phone("123"), "123"); // 过短
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
        );
        assert_eq!(result, "信用卡号：**** **** **** ****，有效期：12/25");
    }
}
