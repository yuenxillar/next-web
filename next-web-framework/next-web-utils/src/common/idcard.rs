
use chrono::{Datelike, NaiveDate, Utc};


/// 身份证工具类
pub struct IdCardUtil;

impl IdCardUtil {

    /// 验证身份证是否合法。
    /// 
    /// # 参数
    /// - `id_card`: 身份证号码（支持15位或18位）。
    /// 
    /// # 返回值
    /// 如果身份证合法，则返回 `true`；否则返回 `false`
    pub fn is_valid_card(id_card: &str) -> bool {
        let id_card = id_card.trim();
        if id_card.len() == 15 {
            // 检查15位身份证的格式
            return id_card.chars().all(|c| c.is_numeric());
        } else if id_card.len() == 18 {
            // 检查18位身份证的格式
            if !id_card[..17].chars().all(|c| c.is_numeric()) {
                return false;
            }
            // 校验最后一位校验码
            return Self::validate_checksum(id_card);
        }
        false
    }

    /// 将15位身份证转换为18位。
    /// 
    /// # 参数
    /// - `id_card`: 15位身份证号码。
    /// 
    /// # 返回值
    /// 返回转换后的18位身份证号码。如果输入不合法，则返回 `None`
    pub fn convert_15_to_18(id_card: &str) -> Option<String> {
        let id_card = id_card.trim();
        if id_card.len() != 15 || !id_card.chars().all(|c| c.is_numeric()) {
            return None;
        }
        // 补全年份
        let full_year = format!("19{}", &id_card[6..12]);
        let new_id_card = format!("{}{}", &id_card[..6], full_year);
        // 计算校验码
        let checksum = Self::calculate_checksum(&new_id_card);
        Some(format!("{}{}", new_id_card, checksum))
    }

    /// 获取身份证中的生日。
    /// 
    /// # 参数
    /// - `id_card`: 身份证号码（支持15位或18位）。
    /// 
    /// # 返回值
    /// 返回生日的 `NaiveDate` 对象。如果身份证不合法，则返回 `None`。
    pub fn get_birth_by_id_card(id_card: &str) -> Option<NaiveDate> {
        let id_card = id_card.trim();
        let birth_str = if id_card.len() == 15 {
            format!("19{}", &id_card[6..12])
        } else if id_card.len() == 18 {
            id_card[6..14].to_string()
        } else {
            return None;
        };
        NaiveDate::parse_from_str(&birth_str, "%Y%m%d").ok()
    }


    /// 获取身份证中的年龄。
    /// 
    /// # 参数
    /// - `id_card`: 身份证号码（支持15位或18位）。
    /// 
    /// # 返回值
    /// 返回年龄。如果身份证不合法，则返回 `None`。
    pub fn get_age_by_id_card(id_card: &str) -> Option<i32> {
        let birth_date = Self::get_birth_by_id_card(id_card)?;
        let now = Utc::now().naive_utc().date();
        let age = now.year() - birth_date.year();
        if now.month() < birth_date.month()
            || (now.month() == birth_date.month() && now.day() < birth_date.day())
        {
            return Some(age - 1);
        }
        Some(age)
    }

    /// 获取身份证中的生日年份。
    /// 
    /// # 参数
    /// - `id_card`: 身份证号码（支持15位或18位）。
    /// 
    /// # 返回值
    /// 返回生日年份。如果身份证不合法，则返回 `None`。
    pub fn get_year_by_id_card(id_card: &str) -> Option<i32> {
        let birth_date = Self::get_birth_by_id_card(id_card)?;
        Some(birth_date.year())
    }

    /// 获取身份证中的生日月份。
    /// 
    /// # 参数
    /// - `id_card`: 身份证号码（支持15位或18位）。
    /// 
    /// # 返回值
    /// 返回生日月份。如果身份证不合法，则返回 `None`。
    pub fn get_month_by_id_card(id_card: &str) -> Option<u32> {
        let birth_date = Self::get_birth_by_id_card(id_card)?;
        Some(birth_date.month())
    }

    /// 获取身份证中的生日日期。
    /// 
    /// # 参数
    /// - `id_card`: 身份证号码（支持15位或18位）。
    /// 
    /// # 返回值
    /// 返回生日日期。如果身份证不合法，则返回 `None`。
    pub fn get_day_by_id_card(id_card: &str) -> Option<u32> {
        let birth_date = Self::get_birth_by_id_card(id_card)?;
        Some(birth_date.day())
    }

    /// 获取身份证中的性别。
    /// 
    /// # 参数
    /// - `id_card`: 身份证号码（支持15位或18位）。
    /// 
    /// # 返回值
    /// 返回性别（"男" 或 "女"）。如果身份证不合法，则返回 `None`。
    pub fn get_gender_by_id_card(id_card: &str) -> Option<&'static str> {
        let id_card = id_card.trim();
        let gender_digit = if id_card.len() == 15 {
            id_card.chars().nth(14)?.to_digit(10)?
        } else if id_card.len() == 18 {
            id_card.chars().nth(16)?.to_digit(10)?
        } else {
            return None;
        };
        if gender_digit % 2 == 0 {
            Some("女")
        } else {
            Some("男")
        }
    }

    /// 获取身份证中的省份代码。
    /// 
    /// # 参数
    /// - `id_card`: 身份证号码（支持15位或18位）。
    /// 
    /// # 返回值
    /// 返回省份代码。如果身份证不合法，则返回 `None`。
    pub fn get_province_by_id_card(id_card: &str) -> Option<&str> {
        let id_card = id_card.trim();
        if id_card.len() != 15 && id_card.len() != 18 {
            return None;
        }
        let province_code = &id_card[..2];
        match province_code {
            "11" => Some("北京市"),
            "12" => Some("天津市"),
            "13" => Some("河北省"),
            "14" => Some("山西省"),
            "15" => Some("内蒙古自治区"),
            "21" => Some("辽宁省"),
            "22" => Some("吉林省"),
            "23" => Some("黑龙江省"),
            "31" => Some("上海市"),
            "32" => Some("江苏省"),
            "33" => Some("浙江省"),
            "34" => Some("安徽省"),
            "35" => Some("福建省"),
            "36" => Some("江西省"),
            "37" => Some("山东省"),
            "41" => Some("河南省"),
            "42" => Some("湖北省"),
            "43" => Some("湖南省"),
            "44" => Some("广东省"),
            "45" => Some("广西壮族自治区"),
            "46" => Some("海南省"),
            "50" => Some("重庆市"),
            "51" => Some("四川省"),
            "52" => Some("贵州省"),
            "53" => Some("云南省"),
            "54" => Some("西藏自治区"),
            "61" => Some("陕西省"),
            "62" => Some("甘肃省"),
            "63" => Some("青海省"),
            "64" => Some("宁夏回族自治区"),
            "65" => Some("新疆维吾尔自治区"),
            _ => None,
        }
    }

    /// 校验18位身份证的校验码是否正确。
    fn validate_checksum(id_card: &str) -> bool {
        const WEIGHTS: [u32; 17] = [7, 9, 10, 5, 8, 4, 2, 1, 6, 3, 7, 9, 10, 5, 8, 4, 2];
        const CHECK_DIGITS: [&str; 11] = ["1", "0", "X", "9", "8", "7", "6", "5", "4", "3", "2"];
        let sum: u32 = id_card[..17]
            .chars()
            .zip(WEIGHTS.iter())
            .map(|(c, w)| c.to_digit(10).unwrap_or(0) * w)
            .sum();
        let mod_value = sum % 11;
        id_card.chars().nth(17).map_or(false, |c| {
            CHECK_DIGITS[mod_value as usize] == c.to_string().to_uppercase()
        })
    }

    /// 计算18位身份证的校验码。
    fn calculate_checksum(id_card: &str) -> char {
        const WEIGHTS: [u32; 17] = [7, 9, 10, 5, 8, 4, 2, 1, 6, 3, 7, 9, 10, 5, 8, 4, 2];
        const CHECK_DIGITS: [char; 11] = ['1', '0', 'X', '9', '8', '7', '6', '5', '4', '3', '2'];
        let sum: u32 = id_card.chars()
            .zip(WEIGHTS.iter())
            .map(|(c, w)| c.to_digit(10).unwrap_or(0) * w)
            .sum();
        let mod_value = sum % 11;
        CHECK_DIGITS[mod_value as usize]
    }
}


#[cfg(test)]
mod id_card_tests {

    use super::*;

    #[test]
    fn test_is_valid_card() {
        assert_eq!(IdCardUtil::is_valid_card("350201198701146613"), true);
    }

    #[test]
    fn test_convert_15_to_18() {
        assert_eq!(IdCardUtil::convert_15_to_18("111111111111111"), Some("111111191111116".to_string()));
    }

    #[test]
    fn test_get_birth_by_id_card() {
        assert_eq!(IdCardUtil::get_birth_by_id_card("350201198701146613"), NaiveDate::from_ymd_opt(1987, 1, 14));
    }

    #[test]
    fn test_get_age_by_id_card() {
        assert_eq!(IdCardUtil::get_age_by_id_card("350201198701146613"), Some(38));
    }

    #[test]
    fn test_get_year_by_id_card() {
        assert_eq!(IdCardUtil::get_year_by_id_card("350201198701146613"), Some(1987));
    }

    #[test]
    fn test_get_month_by_id_card() {
        assert_eq!(IdCardUtil::get_month_by_id_card("350201198701146613"), Some(1));
    }

    #[test]
    fn test_get_day_by_id_card() {
        assert_eq!(IdCardUtil::get_day_by_id_card("350201198701146613"), Some(14));
    }

    #[test]
    fn test_get_gender_by_id_card() {
        assert_eq!(IdCardUtil::get_gender_by_id_card("350201198701146613"), Some("男"));
    }

    #[test]
    fn test_get_province_by_id_card() {
        assert_eq!(IdCardUtil::get_province_by_id_card("350201198701146613"), Some("福建省"));
    }

}