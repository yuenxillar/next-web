
/// 脱敏工具类 / Desensitized util
pub struct DesensitizedUtil;

impl DesensitizedUtil {
    /// 脱敏身份证号码 / Desensitize ID card number
    ///
    /// # 参数 / Parameters
    /// - `id_card`: 身份证号码 / ID card number
    ///
    /// # 返回 / Returns
    /// 脱敏后的身份证号码 / Desensitized ID card number
    ///
    /// # 示例 / Example
    /// ```
    /// assert_eq!(DesensitizedUtil::desensitized_id_card("110105199003071234"), "110105********1234");
    /// ```
    pub fn desensitized_id_card(id_card: &str) -> String {
        if id_card.len() == 18 {
            format!("{}{}{}", &id_card[..6], "********", &id_card[14..])
        } else if id_card.len() == 15 {
            format!("{}{}{}", &id_card[..6], "*****", &id_card[11..])
        } else {
            id_card.to_string()
        }
    }

    /// 中文姓名脱敏 / Chinese name desensitization
    ///
    /// # 参数 / Parameters
    /// - `name`: 中文姓名 / Chinese name
    ///
    /// # 返回 / Returns
    /// 脱敏后的中文姓名，保留姓和最后一个字，中间用*代替（张三→张*，李小明→李*明）  
    /// Desensitized Chinese name, keeping the first and last characters, replacing the middle part with '*'.
    ///
    /// # 示例 / Example
    /// ```
    /// assert_eq!(DesensitizedUtil::desensitized_chinese_name("张三"), "张*");
    /// assert_eq!(DesensitizedUtil::desensitized_chinese_name("李小明"), "李*明");
    /// ```
    pub fn desensitized_chinese_name(name: &str) -> String {
        if name.len() >= 2 {
            let mut result = String::with_capacity(name.len());
            result.push(name.chars().next().unwrap());
            result.extend(std::iter::repeat('*').take(name.len().saturating_sub(2)));
            if name.len() > 2 {
                result.push(name.chars().last().unwrap());
            }
            result
        } else {
            name.to_string()
        }
    }

    /// 手机号脱敏 / Mobile phone desensitization
    ///
    /// # 参数 / Parameters
    /// - `phone`: 手机号 / Mobile phone number
    ///
    /// # 返回 / Returns
    /// 脱敏后的手机号，保留前3位和后4位（13812345678→138****5678）  
    /// Desensitized mobile phone number, keeping the first 3 digits and the last 4 digits.
    ///
    /// # 示例 / Example
    /// ```
    /// assert_eq!(DesensitizedUtil::desensitized_phone("13812345678"), "138****5678");
    /// ```
    pub fn desensitized_phone(phone: &str) -> String {
        if phone.len() >= 11 {
            format!("{}{}{}", &phone[..3], "****", &phone[7..])
        } else {
            phone.to_string()
        }
    }

    /// 地址脱敏 / Address desensitization
    ///
    /// # 参数 / Parameters
    /// - `address`: 地址 / Address
    ///
    /// # 返回 / Returns
    /// 脱敏后的地址，保留前6个字符和后4个字符（北京市朝阳区建国路100号→北京市朝****100号）  
    /// Desensitized address, keeping the first 6 characters and the last 4 characters.
    ///
    /// # 示例 / Example
    /// ```
    /// assert_eq!(DesensitizedUtil::desensitized_address("北京市朝阳区建国路100号"), "北京市朝****100号");
    /// ```
    pub fn desensitized_address(address: &str) -> String {
        if address.len() >= 10 {
            format!("{}{}{}", &address[..6], "****", &address[address.len()-4..])
        } else {
            address.to_string()
        }
    }

    /// 邮箱脱敏 / Email desensitization
    ///
    /// # 参数 / Parameters
    /// - `email`: 邮箱地址 / Email address
    ///
    /// # 返回 / Returns
    /// 脱敏后的邮箱，保留@前1个字符和@后域名（test@example.com→t***@example.com）  
    /// Desensitized email, keeping the first character before '@' and the domain after '@'.
    ///
    /// # 示例 / Example
    /// ```
    /// assert_eq!(DesensitizedUtil::desensitized_email("test@example.com"), "t***@example.com");
    /// ```
    pub fn desensitized_email(email: &str) -> String {
        if let Some(at_index) = email.find('@') {
            if at_index > 1 {
                let prefix = &email[..1];
                let suffix = &email[at_index..];
                format!("{}***{}", prefix, suffix)
            } else {
                email.to_string()
            }
        } else {
            email.to_string()
        }
    }

    /// 银行卡号脱敏 / Bank card desensitization
    ///
    /// # 参数 / Parameters
    /// - `card`: 银行卡号 / Bank card number
    ///
    /// # 返回 / Returns
    /// 脱敏后的银行卡号，保留前6位和后4位（6228480402564890018→622848*********0018）  
    /// Desensitized bank card number, keeping the first 6 digits and the last 4 digits.
    ///
    /// # 示例 / Example
    /// ```
    /// assert_eq!(DesensitizedUtil::desensitized_bank_card("6228480402564890018"), "622848**********0018");
    /// ```
    pub fn desensitized_bank_card(card: &str) -> String {
        if card.len() >= 16 {
            format!("{}{}{}", &card[..6], "**********", &card[card.len()-4..])
        } else {
            card.to_string()
        }
    }

    /// 车牌号脱敏 / License plate desensitization
    ///
    /// # 参数 / Parameters
    /// - `plate`: 车牌号 / License plate number
    ///
    /// # 返回 / Returns
    /// 脱敏后的车牌号，新能源车牌保留前2位和后3位（京AD12345→京*****45）  
    /// Desensitized license plate number, keeping the first 2 characters and the last 3 characters.
    ///
    /// # 示例 / Example
    /// ```
    /// assert_eq!(DesensitizedUtil::desensitized_license_plate("京AD12345"), "京*****45");
    /// ```
    pub fn desensitized_license_plate(plate: &str) -> String {
        if plate.len() >= 7 {
            format!("{}{}{}", &plate[..2], "*****", &plate[plate.len()-3..])
        } else {
            plate.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// 测试身份证号码脱敏 / Test ID card desensitization
    fn test_desensitized_id_card() {
        assert_eq!(DesensitizedUtil::desensitized_id_card("110105199003071234"), "110105********1234");
        assert_eq!(DesensitizedUtil::desensitized_id_card("110105900307123"), "110105*****123");
    }

    #[test]
    /// 测试中文姓名脱敏 / Test Chinese name desensitization
    fn test_desensitized_chinese_name() {
        assert_eq!(DesensitizedUtil::desensitized_chinese_name("张三"), "张*");
        assert_eq!(DesensitizedUtil::desensitized_chinese_name("李小明"), "李*明");
    }

    #[test]
    /// 测试手机号脱敏 / Test mobile phone desensitization
    fn test_desensitized_phone() {
        assert_eq!(DesensitizedUtil::desensitized_phone("13812345678"), "138****5678");
    }

    #[test]
    /// 测试地址脱敏 / Test address desensitization
    fn test_desensitized_address() {
        assert_eq!(DesensitizedUtil::desensitized_address("北京市朝阳区建国路100号"), "北京市朝****100号");
    }

    #[test]
    /// 测试邮箱脱敏 / Test email desensitization
    fn test_desensitized_email() {
        assert_eq!(DesensitizedUtil::desensitized_email("test@example.com"), "t***@example.com");
    }

    #[test]
    /// 测试银行卡号脱敏 / Test bank card desensitization
    fn test_desensitized_bank_card() {
        assert_eq!(DesensitizedUtil::desensitized_bank_card("6228480402564890018"), "622848**********0018");
    }

    #[test]
    /// 测试车牌号脱敏 / Test license plate desensitization
    fn test_desensitized_license_plate() {
        assert_eq!(DesensitizedUtil::desensitized_license_plate("京AD12345"), "京*****45");
    }
}