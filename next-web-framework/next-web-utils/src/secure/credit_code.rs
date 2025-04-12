use std::collections::HashMap;

/// 统一社会信用代码工具类
/// 
/// Unified Social Credit Code Utility
pub struct CreditCodeUtil;

impl CreditCodeUtil {
    /// 验证统一社会信用代码是否有效
    /// 
    /// Verify if the unified social credit code is valid
    pub fn is_valid(code: &str) -> bool {
        if code.len() != 18 {
            return false;
        }

        // 验证登记管理部门代码
        if !Self::is_valid_registration_department(&code[0..1]) {
            return false;
        }

        // 验证机构类别代码
        if !Self::is_valid_organization_type(&code[1..2]) {
            return false;
        }

        // 验证行政区划码
        if !Self::is_valid_region_code(&code[2..8]) {
            return false;
        }

        // 验证主体标识码
        if !Self::is_valid_organization_code(&code[8..17]) {
            return false;
        }

        // 验证校验码
        Self::verify_check_code(code)
    }

    /// 验证登记管理部门代码
    /// 
    /// Verify registration department code
    fn is_valid_registration_department(code: &str) -> bool {
        let valid_codes = [
            '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'Y'
        ];
        valid_codes.contains(&code.chars().next().unwrap())
    }

    /// 验证机构类别代码
    /// 
    /// Verify organization type code
    fn is_valid_organization_type(code: &str) -> bool {
        let valid_codes = [
            '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'J', 'K', 'L', 'M', 'N', 'P', 'Q', 'R'
        ];
        valid_codes.contains(&code.chars().next().unwrap())
    }

    /// 验证行政区划码
    /// 
    /// Verify region code
    fn is_valid_region_code(code: &str) -> bool {
        code.chars().all(|c| c.is_ascii_digit())
    }

    /// 验证主体标识码
    /// 
    /// Verify organization code
    fn is_valid_organization_code(code: &str) -> bool {
        code.chars().all(|c| c.is_ascii_alphanumeric() && !c.is_ascii_lowercase())
    }

    /// 验证校验码
    /// 
    /// Verify check code
    fn verify_check_code(code: &str) -> bool {
        let weights = [1, 3, 9, 27, 19, 26, 16, 17, 20, 29, 25, 13, 8, 24, 10, 30, 28];
        let check_code = code.chars().last().unwrap();
        let mut sum = 0;

        for (i, c) in code[0..17].chars().enumerate() {
            let value = if c.is_ascii_digit() {
                c.to_digit(10).unwrap()
            } else {
                (c as u32) - ('A' as u32) + 10
            };
            sum += value * weights[i];
        }

        let mod_value = 31 - (sum % 31);
        let expected_check_code = if mod_value == 31 {
            '0'
        } else if mod_value < 10 {
            (mod_value as u8 + b'0') as char
        } else {
            (mod_value as u8 - 10 + b'A') as char
        };

        check_code == expected_check_code
    }

    /// 获取登记管理部门名称
    /// 
    /// Get registration department name
    pub fn get_department_name(code: &str) -> Option<&'static str> {
        let department_map: HashMap<char, &str> = [
            ('1', "机构编制"),
            ('2', "外交"),
            ('3', "教育"),
            ('4', "公安"),
            ('5', "民政"),
            ('6', "司法"),
            ('7', "交通运输"),
            ('8', "文化"),
            ('9', "工商"),
            ('A', "中央军委改革和编制办公室"),
            ('B', "中央军委政治工作部"),
            ('C', "中央军委后勤保障部"),
            ('D', "中央军委装备发展部"),
            ('E', "中央军委训练管理部"),
            ('F', "中央军委国防动员部"),
            ('G', "中央军委纪律检查委员会"),
            ('Y', "其他"),
        ].iter().cloned().collect();

        department_map.get(&code.chars().next()?).copied()
    }

    /// 获取机构类别名称
    /// 
    /// Get organization type name
    pub fn get_organization_type_name(code: &str) -> Option<&'static str> {
        let type_map: HashMap<char, &str> = [
            ('1', "企业"),
            ('2', "事业单位"),
            ('3', "社会团体"),
            ('4', "民办非企业单位"),
            ('5', "基金会"),
            ('6', "律师事务所"),
            ('7', "会计师事务所"),
            ('8', "外国企业常驻代表机构"),
            ('9', "外国（地区）企业在中国境内从事生产经营活动"),
            ('A', "农民专业合作社"),
            ('B', "个体工商户"),
            ('C', "其他企业"),
            ('D', "其他事业单位"),
            ('E', "其他社会团体"),
            ('F', "其他民办非企业单位"),
            ('G', "其他基金会"),
            ('H', "其他律师事务所"),
            ('J', "其他会计师事务所"),
            ('K', "其他外国企业常驻代表机构"),
            ('L', "其他外国（地区）企业在中国境内从事生产经营活动"),
            ('M', "其他农民专业合作社"),
            ('N', "其他个体工商户"),
            ('P', "其他"),
        ].iter().cloned().collect();

        type_map.get(&code.chars().nth(1)?).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试有效的统一社会信用代码
    /// 
    /// Test valid unified social credit code
    #[test]
    fn test_valid_credit_code() {
        let valid_codes = [
            "91350100M000100Y43", // 示例代码
            "91110000802100433B", // 示例代码
            "91440300708461136T", // 示例代码
        ];

        for code in valid_codes {
            assert!(CreditCodeUtil::is_valid(code), "Code {} should be valid", code);
        }
    }

    /// 测试无效的统一社会信用代码
    /// 
    /// Test invalid unified social credit code
    #[test]
    fn test_invalid_credit_code() {
        let invalid_codes = [
            "123456789012345678", // 长度错误
            "A23456789012345678", // 无效的登记管理部门代码
            "1A3456789012345678", // 无效的机构类别代码
            "11A345678901234567", // 无效的行政区划码
            "11111111A12345678", // 无效的主体标识码
            "91350100M000100Y44", // 错误的校验码
        ];

        for code in invalid_codes {
            assert!(!CreditCodeUtil::is_valid(code), "Code {} should be invalid", code);
        }
    }

    /// 测试获取登记管理部门名称
    /// 
    /// Test getting registration department name
    #[test]
    fn test_get_department_name() {
        assert_eq!(CreditCodeUtil::get_department_name("91350100M000100Y43"), Some("工商"));
        assert_eq!(CreditCodeUtil::get_department_name("A1350100M000100Y43"), Some("中央军委改革和编制办公室"));
        assert_eq!(CreditCodeUtil::get_department_name("Z1350100M000100Y43"), None);
    }

    /// 测试获取机构类别名称
    /// 
    /// Test getting organization type name
    #[test]
    fn test_get_organization_type_name() {
        assert_eq!(CreditCodeUtil::get_organization_type_name("91350100M000100Y43"), Some("企业"));
        assert_eq!(CreditCodeUtil::get_organization_type_name("91A50100M000100Y43"), Some("农民专业合作社"));
        assert_eq!(CreditCodeUtil::get_organization_type_name("91Z50100M000100Y43"), None);
    }
}