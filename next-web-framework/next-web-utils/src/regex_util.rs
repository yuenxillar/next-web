/**
*struct:    RegexUtil
*desc:      正则表达式工具类
*author:    Listening
*email:     yuenxillar@163.com
*date:      2024/10/02
*/

pub struct RegexUtil;

impl RegexUtil {
    /// check if email is valid
    pub fn is_valid_email(email: &str) -> bool {
        let email_regex =
            regex::Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
        return email_regex.is_match(email);
    }

    /// check if password is valid
    pub fn is_valid_password(password: &str) -> bool {
        let password_regex =
            regex::Regex::new(r"^(?=.*[a-z])(?=.*[A-Z])(?=.*\d)[a-zA-Z\d]{8,}$").unwrap();
        return password_regex.is_match(password);
    }

    pub fn is_valid_phone(phone: &str) -> bool {
        let phone_regex = regex::Regex::new(r"^\+?[0-9]{1,3}\-?[0-9]{3,14}$").unwrap();
        return phone_regex.is_match(phone);
    }
}
