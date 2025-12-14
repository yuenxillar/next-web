use crate::common::{data_source::*, string::StringUtil};

///
pub struct DataGenerator;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerationType {
    // 基本类型
    Email,
    Phone,
    ChineseName,
    EnglishName,
    Job,
    Ip,
    Ipv4,
    Ipv6,
    UserAgent,
    Color,
    Rgb,
    Rgba,
    Hex,

    // 时间日期类型
    Date,
    Time,
    DateTime,
    Timestamp,
    WeekDay,
    Month,
    Year,

    // 文本内容类型
    Word,
    Character,

    // 数字类型
    Number,
    Integer,
    Float,
    NaturalNumber,
    Currency,
    Percentage,

    // ID类型
    UUID,
    ObjectId,

    // 银行卡相关
    BankCard,
    IBAN,

    Latitude,
    Longitude,
    Coordinates,

    // 个人信息
    Age,
    Birthday,
    Avatar,
    IdCard,
    Passport,

    // 网络信息
    Domain,
    Protocol,
    Port,
    Mac,
}

impl DataGenerator {
    pub fn generate(ty: GenerationType) -> String {
        match ty {
            GenerationType::Email => Self::generate_email(),
            GenerationType::Phone => Self::generate_phone(),
            GenerationType::ChineseName => Self::generate_chinese_name(),
            GenerationType::EnglishName => Self::generate_english_name(),
            GenerationType::Job => Self::generate_job(),
            GenerationType::Ip => Self::generate_ip(),
            GenerationType::Ipv4 => Self::generate_ipv4(),
            GenerationType::Ipv6 => Self::generate_ipv6(),
            GenerationType::UserAgent => Self::generate_user_agent(),
            GenerationType::Color => Self::generate_color(),
            GenerationType::Rgb => Self::generate_rgb(),
            GenerationType::Rgba => Self::generate_rgba(),
            GenerationType::Hex => Self::generate_hex(),

            GenerationType::Date => Self::generate_date(),
            GenerationType::Time => Self::generate_time(),
            GenerationType::DateTime => Self::generate_date_time(),
            GenerationType::Timestamp => Self::generate_timestamp(),
            GenerationType::WeekDay => Self::generate_weekday(),
            GenerationType::Month => Self::generate_month(),
            GenerationType::Year => Self::generate_year(),

            GenerationType::Word => Self::generate_word(),
            GenerationType::Character => Self::generate_character(),

            GenerationType::Number => Self::generate_number(),
            GenerationType::Integer => Self::generate_integer(),
            GenerationType::Float => Self::generate_float(),
            GenerationType::NaturalNumber => Self::generate_natural_number(),
            GenerationType::Currency => Self::generate_currency(),
            GenerationType::Percentage => Self::generate_percentage(),

            GenerationType::UUID => Self::generate_uuid(),
            GenerationType::ObjectId => Self::generate_object_id(),

            GenerationType::BankCard => Self::generate_bank_card(),
            GenerationType::IBAN => Self::generate_iban(),

            GenerationType::Latitude => Self::generate_latitude(),
            GenerationType::Longitude => Self::generate_longitude(),
            GenerationType::Coordinates => Self::generate_coordinates(),

            GenerationType::Age => Self::generate_age(),
            GenerationType::Birthday => Self::generate_birthday(),
            GenerationType::Avatar => Self::generate_avatar(),
            GenerationType::IdCard => Self::generate_id_card(),
            GenerationType::Passport => Self::generate_passport(),

            GenerationType::Domain => Self::generate_domain(),
            GenerationType::Protocol => Self::generate_protocol(),
            GenerationType::Port => Self::generate_port(),
            GenerationType::Mac => Self::generate_mac(),
        }
    }

    fn generate_email() -> String {
        let name = StringUtil::generate_random_string(fastrand::usize(5..9));
        let number = fastrand::u32(1000000..9999999);
        let domain = EMAIL_DOMAINS[fastrand::usize(0..EMAIL_DOMAINS.len())];

        format!("{}{}@{}", name, number, domain)
    }

    fn generate_phone() -> String {
        let prefix = MOBILE_PREFIXES[fastrand::usize(0..MOBILE_PREFIXES.len())];

        let mut remaining_digits = String::with_capacity(8);
        for _ in 0..8 {
            remaining_digits.push_str(&fastrand::u8(0..10).to_string());
        }

        format!("{}{}", prefix, remaining_digits)
    }

    fn generate_job() -> String {
        // 生成职位
        let jobs = [
            "软件工程师",
            "产品经理",
            "UI设计师",
            "数据分析师",
            "项目经理",
            "市场专员",
            "销售经理",
            "人力资源专员",
            "财务主管",
            "CEO",
        ];

        jobs[fastrand::usize(0..jobs.len())].to_string()
    }

    fn generate_ip() -> String {
        format!(
            "{}.{}.{}.{}",
            fastrand::u8(1..255),
            fastrand::u8(0..255),
            fastrand::u8(0..255),
            fastrand::u8(1..255)
        )
    }

    fn generate_user_agent() -> String {
        // 生成User Agent
        let user_agents = [
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/14.1.1 Safari/605.1.15",
            "Mozilla/5.0 (iPhone; CPU iPhone OS 14_6 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/14.0 Mobile/15E148 Safari/604.1",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:89.0) Gecko/20100101 Firefox/89.0",
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.114 Safari/537.36"
        ];

        user_agents[fastrand::usize(0..user_agents.len())].to_string()
    }

    fn generate_color() -> String {
        format!(
            "#{:02X}{:02X}{:02X}",
            fastrand::u8(0..255),
            fastrand::u8(0..255),
            fastrand::u8(0..255)
        )
    }

    // 生成日期 YYYY-MM-DD
    fn generate_date() -> String {
        let year = fastrand::u32(1970..2300);
        let month = fastrand::u32(1..13);
        // 简化处理，避免处理不同月份的天数
        let day = fastrand::u32(1..29);

        format!("{:04}-{:02}-{:02}", year, month, day)
    }

    fn generate_time() -> String {
        // 生成时间 HH:MM:SS
        let hour = fastrand::u32(0..24);
        let minute = fastrand::u32(0..60);
        let second = fastrand::u32(0..60);

        format!("{:02}:{:02}:{:02}", hour, minute, second)
    }

    /// 生成日期时间 YYYY-MM-DD HH:MM:SS
    fn generate_date_time() -> String {
        format!("{} {}", Self::generate_date(), Self::generate_time())
    }

    fn generate_chinese_name() -> String {
        let first = COUNTRIES[fastrand::usize(0..COUNTRIES.len())];
        let last = CHINESE_NAMES[fastrand::usize(0..CHINESE_NAMES.len())];

        format!("{}{}", first, last)
    }

    fn generate_english_name() -> String {
        let first = FIRST_NAMES[fastrand::usize(0..FIRST_NAMES.len())];
        let last = LAST_NAMES[fastrand::usize(0..LAST_NAMES.len())];

        format!("{} {}", first, last)
    }

    fn generate_ipv4() -> String {
        format!(
            "{}.{}.{}.{}",
            fastrand::u8(1..255),
            fastrand::u8(0..255),
            fastrand::u8(0..255),
            fastrand::u8(1..255)
        )
    }

    fn generate_ipv6() -> String {
        format!(
            "{:04x}:{:04x}:{:04x}:{:04x}:{:04x}:{:04x}:{:04x}:{:04x}",
            fastrand::u16(0..65535),
            fastrand::u16(0..65535),
            fastrand::u16(0..65535),
            fastrand::u16(0..65535),
            fastrand::u16(0..65535),
            fastrand::u16(0..65535),
            fastrand::u16(0..65535),
            fastrand::u16(0..65535)
        )
    }

    fn generate_rgb() -> String {
        format!(
            "rgb({}, {}, {})",
            fastrand::u8(0..255),
            fastrand::u8(0..255),
            fastrand::u8(0..255)
        )
    }

    fn generate_rgba() -> String {
        format!(
            "rgba({}, {}, {}, {:.1})",
            fastrand::u8(0..255),
            fastrand::u8(0..255),
            fastrand::u8(0..255),
            (fastrand::u8(0..10) as f32) / 10.0
        )
    }

    fn generate_hex() -> String {
        format!(
            "#{:02X}{:02X}{:02X}",
            fastrand::u8(0..255),
            fastrand::u8(0..255),
            fastrand::u8(0..255)
        )
    }

    fn generate_timestamp() -> String {
        // 生成时间戳 (2000-01-01至今的秒数)
        let seconds_since_2000 =
            fastrand::u64(0..chrono::Utc::now().timestamp() as u64 - 946684800); // 946684800是2000-01-01的时间戳
        (seconds_since_2000 + 946684800).to_string()
    }

    fn generate_weekday() -> String {
        let weekdays = [
            "星期一",
            "星期二",
            "星期三",
            "星期四",
            "星期五",
            "星期六",
            "星期日",
        ];
        weekdays[fastrand::usize(0..weekdays.len())].to_string()
    }

    fn generate_month() -> String {
        let months = [
            "一月",
            "二月",
            "三月",
            "四月",
            "五月",
            "六月",
            "七月",
            "八月",
            "九月",
            "十月",
            "十一月",
            "十二月",
        ];
        months[fastrand::usize(0..months.len())].to_string()
    }

    fn generate_year() -> String {
        fastrand::u32(1970..2200).to_string()
    }

    fn generate_word() -> String {
        let words = [
            "科技",
            "创新",
            "发展",
            "未来",
            "智能",
            "数据",
            "互联网",
            "算法",
            "安全",
            "效率",
            "体验",
            "设计",
            "质量",
            "服务",
            "管理",
            "优化",
            "系统",
            "平台",
            "资源",
            "工具",
            "方案",
            "策略",
            "模式",
            "理念",
            "technology",
            "innovation",
            "development",
            "future",
            "intelligent",
            "data",
            "internet",
            "algorithm",
            "security",
            "efficiency",
            "experience",
            "design",
            "quality",
            "service",
            "management",
        ];

        words[fastrand::usize(0..words.len())].to_string()
    }

    fn generate_character() -> String {
        let chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789我你他她它们我们你们的地得和是在有个一不了";
        let char_bytes = chars.as_bytes();

        let index = fastrand::usize(0..char_bytes.len());
        if char_bytes[index] < 128 {
            // ASCII字符
            (char_bytes[index] as char).to_string()
        } else {
            // 中文字符 (UTF-8编码，需要处理多字节)
            let mut chars = chars.chars();
            for _ in 0..index {
                chars.next();
            }
            chars.next().unwrap().to_string()
        }
    }

    fn generate_number() -> String {
        if fastrand::bool() {
            // 整数
            fastrand::i64(-1000000..1000000).to_string()
        } else {
            // 浮点数
            format!("{:.2}", fastrand::f64() * 1000.0 - 500.0)
        }
    }

    fn generate_integer() -> String {
        fastrand::i64(-1000000..1000000).to_string()
    }

    fn generate_float() -> String {
        format!("{:.4}", fastrand::f64() * 1000.0 - 500.0)
    }

    fn generate_natural_number() -> String {
        fastrand::u64(1..1000000).to_string()
    }

    fn generate_currency() -> String {
        let currencies = ["CNY", "USD", "EUR", "GBP", "JPY"];
        let currency = currencies[fastrand::usize(0..currencies.len())];
        let amount = format!("{:.2}", fastrand::f64() * 10000.0);

        format!("{} {}", amount, currency)
    }

    fn generate_percentage() -> String {
        format!("{}%", fastrand::u32(0..100))
    }

    fn generate_uuid() -> String {
        format!(
            "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
            fastrand::u32(0..0xFFFFFFFF),
            fastrand::u16(0..0xFFFF),
            (0x4000 | (fastrand::u16(0..0x0FFF))), // 设置版本4的UUID
            (0x8000 | (fastrand::u16(0..0x3FFF))), // 设置变体
            fastrand::u64(0..0xFFFFFFFFFFFF)
        )
    }

    fn generate_object_id() -> String {
        // MongoDB ObjectId格式: 4字节时间戳 + 3字节机器标识 + 2字节进程ID + 3字节计数器
        let timestamp = format!("{:08x}", chrono::Utc::now().timestamp() as u32);
        let machine_id = format!("{:06x}", fastrand::u32(0..0xFFFFFF));
        let process_id = format!("{:04x}", fastrand::u16(0..0xFFFF));
        let counter = format!("{:06x}", fastrand::u32(0..0xFFFFFF));

        format!("{}{}{}{}", timestamp, machine_id, process_id, counter)
    }

    fn generate_bank_card() -> String {
        // 生成16位银行卡号
        let prefixes = ["4", "5", "6"]; // Visa, Mastercard, etc.
        let prefix = prefixes[fastrand::usize(0..prefixes.len())];

        let mut number = prefix.to_string();
        for _ in 0..(16 - prefix.len()) {
            number.push_str(&fastrand::u32(0..10).to_string());
        }

        number
    }

    fn generate_iban() -> String {
        // 简化的IBAN (国际银行账号)
        let country_codes = ["CN", "US", "GB", "DE", "FR", "JP"];
        let country = country_codes[fastrand::usize(0..country_codes.len())];

        let mut acc_number = String::new();
        for _ in 0..20 {
            acc_number.push_str(&fastrand::u32(0..10).to_string());
        }

        format!("{}{}", country, acc_number)
    }

    fn generate_latitude() -> String {
        // 纬度范围: -90到90
        format!("{:.6}", (fastrand::f64() * 180.0) - 90.0)
    }

    fn generate_longitude() -> String {
        // 经度范围: -180到180
        format!("{:.6}", (fastrand::f64() * 360.0) - 180.0)
    }

    fn generate_coordinates() -> String {
        format!(
            "({}, {})",
            Self::generate_latitude(),
            Self::generate_longitude()
        )
    }

    fn generate_age() -> String {
        fastrand::u32(1..100).to_string()
    }

    fn generate_birthday() -> String {
        let year = fastrand::u32(1950..2999);
        let month = fastrand::u32(1..13);
        let day = fastrand::u32(1..29);

        format!("{:04}-{:02}-{:02}", year, month, day)
    }

    fn generate_avatar() -> String {
        let sizes = ["50x50", "100x100", "200x200", "300x300"];
        let size = sizes[fastrand::usize(0..sizes.len())];
        format!("https://i.pravatar.cc/{}", size)
    }

    fn generate_id_card() -> String {
        // 6位地区码 + 8位出生日期 + 3位序列号 + 1位校验码
        let area_code = format!("{:06}", fastrand::u32(110000..700000));

        let year = fastrand::u32(1950..2010);
        let month = fastrand::u32(1..13);
        let day = fastrand::u32(1..29);
        let date = format!("{:04}{:02}{:02}", year, month, day);

        let sequence = format!("{:03}", fastrand::u32(0..1000));

        // 校验码（简化处理，实际应按GB 11643-1999标准计算）
        let check_code = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'X'];
        let check = check_code[fastrand::usize(0..check_code.len())];

        format!("{}{}{}{}", area_code, date, sequence, check)
    }

    fn generate_passport() -> String {
        // 护照号：1-2位字母 + 7-8位数字
        let letters = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let letter1 = letters.chars().nth(fastrand::usize(0..26)).unwrap();
        let letter2 = letters.chars().nth(fastrand::usize(0..26)).unwrap();

        let mut number = String::new();
        for _ in 0..7 {
            number.push_str(&fastrand::u32(0..10).to_string());
        }

        format!("{}{}{}", letter1, letter2, number)
    }

    fn generate_domain() -> String {
        let names = [
            "example", "test", "demo", "mysite", "website", "company", "project", "app",
        ];
        let tlds = [".com", ".org", ".net", ".io", ".tech", ".cn", ".co", ".me"];

        let name = names[fastrand::usize(0..names.len())];
        let tld = tlds[fastrand::usize(0..tlds.len())];

        format!("{}{}", name, tld)
    }

    fn generate_protocol() -> String {
        PROTOCOLS[fastrand::usize(0..PROTOCOLS.len())].to_string()
    }

    fn generate_port() -> String {
        // 常用端口
        let common_ports = [21, 22, 25, 80, 443, 3306, 6379, 8080, 8443, 27017];
        if fastrand::bool() {
            common_ports[fastrand::usize(0..common_ports.len())].to_string()
        } else {
            // 随机端口
            fastrand::u16(1024..65535).to_string()
        }
    }

    fn generate_mac() -> String {
        format!(
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            fastrand::u8(0..255),
            fastrand::u8(0..255),
            fastrand::u8(0..255),
            fastrand::u8(0..255),
            fastrand::u8(0..255),
            fastrand::u8(0..255)
        )
    }
}
