pub struct MockData;


pub enum MockDataType {
    // 基本类型
    Email,
    Phone,
    Name,
    ChineseName,
    EnglishName,
    Address,
    ChineseAddress,
    Company,
    Job,
    Website,
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
    Json,
    Html,
    Text,
    Paragraph,
    Sentence,
    Word,
    Character,
    Title,
    
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
    BankName,
    
    // 地理位置
    Country,
    Province,
    City,
    Latitude,
    Longitude,
    Coordinates,
    
    // 个人信息
    Gender,
    Age,
    Birthday,
    Avatar,
    IdCard,
    Passport,
    
    // 网络信息
    Domain,
    Protocol,
    Port,
    Url,
    Mac,
}


impl MockData {
    pub fn generate(data_type: MockDataType) -> String {
        match data_type {
            MockDataType::Email => Self::generate_email(),
            MockDataType::Phone => Self::generate_phone(),
            MockDataType::Name => Self::generate_name(),
            MockDataType::ChineseName => Self::generate_chinese_name(),
            MockDataType::EnglishName => Self::generate_english_name(),
            MockDataType::Address => Self::generate_address(),
            MockDataType::ChineseAddress => Self::generate_chinese_address(),
            MockDataType::Company => Self::generate_company(),
            MockDataType::Job => Self::generate_job(),
            MockDataType::Website => Self::generate_website(),
            MockDataType::Ip => Self::generate_ip(),
            MockDataType::Ipv4 => Self::generate_ipv4(),
            MockDataType::Ipv6 => Self::generate_ipv6(),
            MockDataType::UserAgent => Self::generate_user_agent(),
            MockDataType::Color => Self::generate_color(),
            MockDataType::Rgb => Self::generate_rgb(),
            MockDataType::Rgba => Self::generate_rgba(),
            MockDataType::Hex => Self::generate_hex(),
            
            MockDataType::Date => Self::generate_date(),
            MockDataType::Time => Self::generate_time(),
            MockDataType::DateTime => Self::generate_date_time(),
            MockDataType::Timestamp => Self::generate_timestamp(),
            MockDataType::WeekDay => Self::generate_weekday(),
            MockDataType::Month => Self::generate_month(),
            MockDataType::Year => Self::generate_year(),
            
            MockDataType::Json => Self::generate_json(),
            MockDataType::Html => Self::generate_html(),
            MockDataType::Text => Self::generate_text(),
            MockDataType::Paragraph => Self::generate_paragraph(),
            MockDataType::Sentence => Self::generate_sentence(),
            MockDataType::Word => Self::generate_word(),
            MockDataType::Character => Self::generate_character(),
            MockDataType::Title => Self::generate_title(),
            
            MockDataType::Number => Self::generate_number(),
            MockDataType::Integer => Self::generate_integer(),
            MockDataType::Float => Self::generate_float(),
            MockDataType::NaturalNumber => Self::generate_natural_number(),
            MockDataType::Currency => Self::generate_currency(),
            MockDataType::Percentage => Self::generate_percentage(),
            
            MockDataType::UUID => Self::generate_uuid(),
            MockDataType::ObjectId => Self::generate_object_id(),
            
            MockDataType::BankCard => Self::generate_bank_card(),
            MockDataType::IBAN => Self::generate_iban(),
            MockDataType::BankName => Self::generate_bank_name(),
            
            MockDataType::Country => Self::generate_country(),
            MockDataType::Province => Self::generate_province(),
            MockDataType::City => Self::generate_city(),
            MockDataType::Latitude => Self::generate_latitude(),
            MockDataType::Longitude => Self::generate_longitude(),
            MockDataType::Coordinates => Self::generate_coordinates(),
            
            MockDataType::Gender => Self::generate_gender(),
            MockDataType::Age => Self::generate_age(),
            MockDataType::Birthday => Self::generate_birthday(),
            MockDataType::Avatar => Self::generate_avatar(),
            MockDataType::IdCard => Self::generate_id_card(),
            MockDataType::Passport => Self::generate_passport(),
            
            MockDataType::Domain => Self::generate_domain(),
            MockDataType::Protocol => Self::generate_protocol(),
            MockDataType::Port => Self::generate_port(),
            MockDataType::Url => Self::generate_url(),
            MockDataType::Mac => Self::generate_mac(),
        }
    }

    fn generate_email() -> String {
        // 生成随机邮箱
        let domains = ["gmail.com", "yahoo.com", "outlook.com", "163.com", "qq.com"];
        let names = ["user", "john", "jane", "alice", "bob", "zhang", "li", "wang"];
        let numbers = ["123", "456", "789", "2023", "2024"];
        
        let name = names[fastrand::usize(0..names.len())];
        let number = numbers[fastrand::usize(0..numbers.len())];
        let domain = domains[fastrand::usize(0..domains.len())];
        
        format!("{}{}@{}", name, number, domain)
    }

    fn generate_phone() -> String {
        // 生成中国手机号
        let prefixes = ["138", "139", "150", "151", "152", "157", "158", "159", "186", "188", "189"];
        let prefix = prefixes[fastrand::usize(0..prefixes.len())];
        let mut number = prefix.to_string();
        
        for _ in 0..8 {
            number.push_str(&fastrand::u32(0..10).to_string());
        }
        
        number
    }

    fn generate_name() -> String {
        // 生成中英文姓名
        let first_names = ["张", "王", "李", "赵", "刘", "John", "Jane", "Mike", "Sarah", "Tom"];
        let last_names = ["伟", "芳", "娜", "强", "Smith", "Johnson", "Williams", "Brown", "Jones"];
        
        let first = first_names[fastrand::usize(0..first_names.len())];
        let last = last_names[fastrand::usize(0..last_names.len())];
        
        format!("{}{}", first, last)
    }

    fn generate_address() -> String {
        // 生成地址
        let provinces = ["北京市", "上海市", "广东省", "江苏省", "浙江省"];
        let cities = ["朝阳区", "浦东新区", "广州市", "南京市", "杭州市"];
        let streets = ["人民路", "中山路", "解放路", "建国路", "长安街"];
        let numbers = ["88号", "123号", "256号", "789号", "1001号"];
        
        let province = provinces[fastrand::usize(0..provinces.len())];
        let city = cities[fastrand::usize(0..cities.len())];
        let street = streets[fastrand::usize(0..streets.len())];
        let number = numbers[fastrand::usize(0..numbers.len())];
        
        format!("{}{}{}{}大厦", province, city, street, number)
    }

    fn generate_company() -> String {
        // 生成公司名称
        let prefixes = ["华为", "阿里", "腾讯", "百度", "京东", "小米", "谷歌", "苹果"];
        let suffixes = ["科技有限公司", "网络有限公司", "信息技术有限公司", "软件开发有限公司"];
        
        let prefix = prefixes[fastrand::usize(0..prefixes.len())];
        let suffix = suffixes[fastrand::usize(0..suffixes.len())];
        
        format!("{}{}", prefix, suffix)
    }

    fn generate_job() -> String {
        // 生成职位
        let jobs = [
            "软件工程师", "产品经理", "UI设计师", "数据分析师", "项目经理",
            "市场专员", "销售经理", "人力资源专员", "财务主管", "CEO"
        ];
        
        jobs[fastrand::usize(0..jobs.len())].to_string()
    }

    fn generate_website() -> String {
        // 生成网站URL
        let protocols = ["http://", "https://"];
        let domains = ["example", "test", "demo", "sample", "my", "cool", "new"];
        let tlds = [".com", ".org", ".net", ".cn", ".io", ".tech"];
        
        let protocol = protocols[fastrand::usize(0..protocols.len())];
        let domain = domains[fastrand::usize(0..domains.len())];
        let tld = tlds[fastrand::usize(0..tlds.len())];
        
        format!("{}{}{}", protocol, domain, tld)
    }

    fn generate_ip() -> String {
        // 生成IP地址
        format!("{}.{}.{}.{}", 
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
        // 生成颜色代码
        format!("#{:02X}{:02X}{:02X}", 
            fastrand::u8(0..255), 
            fastrand::u8(0..255), 
            fastrand::u8(0..255)
        )
    }

    fn generate_date() -> String {
        // 生成日期 YYYY-MM-DD
        let year = fastrand::u32(2000..2030);
        let month = fastrand::u32(1..13);
        let day = fastrand::u32(1..29); // 简化处理，避免处理不同月份的天数
        
        format!("{:04}-{:02}-{:02}", year, month, day)
    }

    fn generate_time() -> String {
        // 生成时间 HH:MM:SS
        let hour = fastrand::u32(0..24);
        let minute = fastrand::u32(0..60);
        let second = fastrand::u32(0..60);
        
        format!("{:02}:{:02}:{:02}", hour, minute, second)
    }

    fn generate_date_time() -> String {
        // 生成日期时间 YYYY-MM-DD HH:MM:SS
        format!("{} {}", Self::generate_date(), Self::generate_time())
    }

    fn generate_json() -> String {
        // 生成简单的JSON数据
        format!(r#"{{
  "id": {},
  "name": "{}",
  "email": "{}",
  "active": {},
  "created_at": "{}"
}}"#, 
            fastrand::u32(1000..10000),
            Self::generate_name(),
            Self::generate_email(),
            fastrand::bool(),
            Self::generate_date_time()
        )
    }

    fn generate_html() -> String {
        // 生成简单的HTML页面
        format!(r#"<!DOCTYPE html>
<html>
<head>
  <title>Mock HTML Page</title>
</head>
<body>
  <h1>欢迎访问</h1>
  <p>这是一个由MockData生成的HTML页面。</p>
  <p>当前时间: {}</p>
  <div>
    <a href="{}">示例链接</a>
  </div>
</body>
</html>"#,
            Self::generate_date_time(),
            Self::generate_website()
        )
    }

    fn generate_text() -> String {
        // 生成随机文本
        let paragraphs = [
            "这是一段示例文本，用于测试。",
            "Lorem ipsum dolor sit amet, consectetur adipiscing elit.",
            "数据模拟是软件开发中的重要环节，可以帮助开发者在不依赖真实数据的情况下进行测试。",
            "人工智能技术正在快速发展，改变着我们的生活和工作方式。",
            "The quick brown fox jumps over the lazy dog."
        ];
        
        let mut text = String::new();
        let count = fastrand::usize(1..4);
        
        for _ in 0..count {
            text.push_str(paragraphs[fastrand::usize(0..paragraphs.len())]);
            text.push_str("\n\n");
        }
        
        text.trim_end().to_string()
    }

    fn generate_chinese_name() -> String {
        let first_names = ["张", "王", "李", "赵", "刘", "陈", "杨", "黄", "周", "吴", "徐", "孙", "马", "朱", "胡", "林", "郭", "何", "高", "罗"];
        let last_names = ["伟", "芳", "娜", "秀英", "敏", "静", "丽", "强", "磊", "军", "洋", "勇", "艳", "杰", "涛", "明", "超", "霞", "平", "刚"];
        
        let first = first_names[fastrand::usize(0..first_names.len())];
        let last = last_names[fastrand::usize(0..last_names.len())];
        
        format!("{}{}", first, last)
    }
    
    fn generate_english_name() -> String {
        let first_names = ["John", "Jane", "Michael", "Emily", "David", "Sarah", "Robert", "Emma", "William", "Olivia", 
                         "James", "Sophia", "Thomas", "Ava", "Charles", "Isabella", "Daniel", "Mia", "Matthew", "Charlotte"];
        let last_names = ["Smith", "Johnson", "Williams", "Brown", "Jones", "Garcia", "Miller", "Davis", "Rodriguez", "Martinez", 
                         "Wilson", "Anderson", "Taylor", "Thomas", "Moore", "Jackson", "Martin", "Lee", "Thompson", "White"];
        
        let first = first_names[fastrand::usize(0..first_names.len())];
        let last = last_names[fastrand::usize(0..last_names.len())];
        
        format!("{} {}", first, last)
    }
    
    fn generate_chinese_address() -> String {
        let provinces = ["北京市", "上海市", "广东省", "江苏省", "浙江省", "四川省", "湖北省", "河南省", "山东省", "河北省"];
        let cities = ["北京市", "上海市", "广州市", "深圳市", "南京市", "杭州市", "成都市", "武汉市", "郑州市", "济南市", "石家庄市"];
        let districts = ["朝阳区", "海淀区", "浦东新区", "天河区", "福田区", "玄武区", "西湖区", "武侯区", "江汉区", "金水区"];
        let streets = ["人民路", "中山路", "解放路", "建国路", "长安街", "东方路", "光明街", "和平大道", "新华路", "金融街"];
        let communities = ["锦绣花园", "阳光小区", "幸福家园", "和谐社区", "未来城", "翡翠湾", "御景园", "紫荆花园", "金色家园", "望江苑"];
        let building_numbers = ["1号楼", "2号楼", "A座", "B座", "3号楼", "C座", "5号楼", "8号楼", "10号楼", "12号楼"];
        let unit_numbers = ["1单元", "2单元", "3单元", "4单元", "5单元", "6单元", "7单元", "8单元"];
        let room_numbers = ["101室", "202室", "303室", "405室", "506室", "607室", "801室", "902室", "1001室", "1103室"];
        
        let province = provinces[fastrand::usize(0..provinces.len())];
        let city = cities[fastrand::usize(0..cities.len())];
        let district = districts[fastrand::usize(0..districts.len())];
        let street = streets[fastrand::usize(0..streets.len())];
        let community = communities[fastrand::usize(0..communities.len())];
        let building = building_numbers[fastrand::usize(0..building_numbers.len())];
        let unit = unit_numbers[fastrand::usize(0..unit_numbers.len())];
        let room = room_numbers[fastrand::usize(0..room_numbers.len())];
        
        format!("{}{}{}{}{}号{}{}{}",
            province, city, district, street, 
            fastrand::u32(1..200), community, building, room)
    }

    fn generate_ipv4() -> String {
        format!("{}.{}.{}.{}", 
            fastrand::u8(1..255), 
            fastrand::u8(0..255), 
            fastrand::u8(0..255), 
            fastrand::u8(1..255))
    }
    
    fn generate_ipv6() -> String {
        format!("{:04x}:{:04x}:{:04x}:{:04x}:{:04x}:{:04x}:{:04x}:{:04x}",
            fastrand::u16(0..65535),
            fastrand::u16(0..65535),
            fastrand::u16(0..65535),
            fastrand::u16(0..65535),
            fastrand::u16(0..65535),
            fastrand::u16(0..65535),
            fastrand::u16(0..65535),
            fastrand::u16(0..65535))
    }
    
    fn generate_rgb() -> String {
        format!("rgb({}, {}, {})",
            fastrand::u8(0..255),
            fastrand::u8(0..255),
            fastrand::u8(0..255))
    }
    
    fn generate_rgba() -> String {
        format!("rgba({}, {}, {}, {:.1})",
            fastrand::u8(0..255),
            fastrand::u8(0..255),
            fastrand::u8(0..255),
            (fastrand::u8(0..10) as f32) / 10.0)
    }
    
    fn generate_hex() -> String {
        format!("#{:02X}{:02X}{:02X}",
            fastrand::u8(0..255),
            fastrand::u8(0..255),
            fastrand::u8(0..255))
    }
    
    fn generate_timestamp() -> String {
        // 生成时间戳 (2000-01-01至今的秒数)
        let seconds_since_2000 = fastrand::u64(0..
            chrono::Utc::now().timestamp() as u64 - 946684800); // 946684800是2000-01-01的时间戳
        (seconds_since_2000 + 946684800).to_string()
    }
    
    fn generate_weekday() -> String {
        let weekdays = ["星期一", "星期二", "星期三", "星期四", "星期五", "星期六", "星期日"];
        weekdays[fastrand::usize(0..weekdays.len())].to_string()
    }
    
    fn generate_month() -> String {
        let months = ["一月", "二月", "三月", "四月", "五月", "六月", "七月", "八月", "九月", "十月", "十一月", "十二月"];
        months[fastrand::usize(0..months.len())].to_string()
    }
    
    fn generate_year() -> String {
        fastrand::u32(1900..2030).to_string()
    }
    
    fn generate_paragraph() -> String {
        let paragraphs = [
            "数据模拟是软件开发中的重要环节，可以帮助开发者在不依赖真实数据的情况下进行测试。通过生成符合特定格式和规则的随机数据，开发者可以验证系统的功能、性能和安全性。这种方法特别适用于初期开发和持续集成环境，能够加速开发流程并提高代码质量。",
            "在人工智能领域，大型语言模型正在改变我们与计算机交互的方式。这些模型通过分析海量文本数据学习语言规律，能够生成连贯、有意义的文本内容，并理解复杂的语境和指令。未来，随着技术的不断进步，这些模型将变得更加智能和自然，进一步拓展人机交互的边界。",
            "云计算技术为企业提供了灵活、可扩展的IT资源，帮助企业降低基础设施成本，提高运营效率。从最初的基础设施即服务(IaaS)，到平台即服务(PaaS)和软件即服务(SaaS)，云计算的服务模式不断丰富和完善。如今，混合云和多云战略成为主流，为企业数字化转型提供了强大支持。",
            "区块链技术通过去中心化的分布式账本系统，为数据的安全和透明提供了新的解决方案。这种技术不仅应用于加密货币，还可用于供应链管理、身份验证、智能合约等多个领域。尽管区块链仍面临扩展性和能源消耗等挑战，但其潜力正被越来越多的行业所认可。",
            "移动互联网的普及改变了人们的生活方式和消费习惯。智能手机成为人们获取信息、社交交流、娱乐消费的主要入口。移动支付、即时通讯、短视频等应用深刻影响了社会经济结构和文化形态。未来，随着5G技术的广泛应用，移动互联网将迎来新一轮的革新和发展。"
        ];
        
        paragraphs[fastrand::usize(0..paragraphs.len())].to_string()
    }
    
    fn generate_sentence() -> String {
        let sentences = [
            "春天来了，花儿开了，小鸟在枝头欢快地歌唱。",
            "科技的发展极大地改变了我们的生活方式。",
            "学习是一个持续不断的过程，需要我们保持好奇心和开放的心态。",
            "健康的生活方式包括均衡的饮食、适当的运动和充足的休息。",
            "团队合作是现代工作环境中不可或缺的重要能力。",
            "数据安全和隐私保护变得越来越重要。",
            "阅读优秀的文学作品可以拓展我们的视野，丰富我们的内心世界。",
            "环境保护需要每个人的共同努力和责任担当。",
            "文化的多样性是人类社会的宝贵财富。",
            "批判性思维能力有助于我们做出更理性的判断和决策。"
        ];
        
        sentences[fastrand::usize(0..sentences.len())].to_string()
    }
    
    fn generate_word() -> String {
        let words = [
            "科技", "创新", "发展", "未来", "智能", "数据", "互联网", "算法",
            "安全", "效率", "体验", "设计", "质量", "服务", "管理", "优化",
            "系统", "平台", "资源", "工具", "方案", "策略", "模式", "理念",
            "technology", "innovation", "development", "future", "intelligent",
            "data", "internet", "algorithm", "security", "efficiency",
            "experience", "design", "quality", "service", "management"
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
    
    fn generate_title() -> String {
        let titles = [
            "未来科技发展趋势分析",
            "人工智能在医疗领域的应用",
            "数字化转型：挑战与机遇",
            "云计算技术的最新进展",
            "物联网如何改变我们的生活",
            "大数据分析与商业决策",
            "区块链技术及其潜在价值",
            "网络安全防护的最佳实践",
            "移动互联网时代的用户体验设计",
            "软件开发中的敏捷方法论"
        ];
        
        titles[fastrand::usize(0..titles.len())].to_string()
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
        format!("{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
            fastrand::u32(0..0xFFFFFFFF),
            fastrand::u16(0..0xFFFF),
            (0x4000 | (fastrand::u16(0..0x0FFF))), // 设置版本4的UUID
            (0x8000 | (fastrand::u16(0..0x3FFF))), // 设置变体
            fastrand::u64(0..0xFFFFFFFFFFFF))
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
    
    fn generate_bank_name() -> String {
        let banks = [
            "中国工商银行", "中国农业银行", "中国银行", "中国建设银行", "交通银行",
            "中国邮政储蓄银行", "招商银行", "浦发银行", "中信银行", "光大银行",
            "Bank of America", "Chase Bank", "HSBC", "Citibank", "Wells Fargo",
            "Deutsche Bank", "BNP Paribas", "Barclays", "Santander", "UBS"
        ];
        
        banks[fastrand::usize(0..banks.len())].to_string()
    }
    
    fn generate_country() -> String {
        let countries = [
            "中国", "美国", "英国", "德国", "法国", "日本", "韩国", "俄罗斯", "加拿大", "澳大利亚",
            "巴西", "印度", "意大利", "西班牙", "新加坡", "马来西亚", "泰国", "越南", "墨西哥", "埃及"
        ];
        
        countries[fastrand::usize(0..countries.len())].to_string()
    }
    
    fn generate_province() -> String {
        let provinces = [
            "北京市", "上海市", "天津市", "重庆市", "河北省", "山西省", "辽宁省", "吉林省",
            "黑龙江省", "江苏省", "浙江省", "安徽省", "福建省", "江西省", "山东省", "河南省",
            "湖北省", "湖南省", "广东省", "海南省", "四川省", "贵州省", "云南省", "陕西省",
            "甘肃省", "青海省", "台湾省", "内蒙古自治区", "广西壮族自治区", "西藏自治区",
            "宁夏回族自治区", "新疆维吾尔自治区", "香港特别行政区", "澳门特别行政区"
        ];
        
        provinces[fastrand::usize(0..provinces.len())].to_string()
    }
    
    fn generate_city() -> String {
        let cities = [
            "北京", "上海", "广州", "深圳", "杭州", "南京", "武汉", "成都", "重庆", "西安",
            "天津", "苏州", "郑州", "长沙", "东莞", "沈阳", "青岛", "宁波", "昆明", "大连",
            "厦门", "福州", "无锡", "合肥", "济南", "哈尔滨", "长春", "贵阳", "南宁", "南昌"
        ];
        
        cities[fastrand::usize(0..cities.len())].to_string()
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
        format!("({}, {})", Self::generate_latitude(), Self::generate_longitude())
    }
    
    fn generate_gender() -> String {
        let genders = ["Man", "Woman", "Male", "Female"];
        genders[fastrand::usize(0..genders.len())].to_string()
    }
    
    fn generate_age() -> String {
        fastrand::u32(1..100).to_string()
    }
    
    fn generate_birthday() -> String {
        let year = fastrand::u32(1950..2999);
        let month = fastrand::u32(1..13);
        let day = fastrand::u32(1..29); // 简化处理，避免处理不同月份的天数
        
        format!("{:04}-{:02}-{:02}", year, month, day)
    }
    
    fn generate_avatar() -> String {
        let sizes = ["50x50", "100x100", "200x200", "300x300"];
        let size = sizes[fastrand::usize(0..sizes.len())];
        format!("https://i.pravatar.cc/{}", size)
    }
    
    fn generate_id_card() -> String {
        // 简化的中国身份证号生成
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
        let names = ["example", "test", "demo", "mysite", "website", "company", "project", "app"];
        let tlds = [".com", ".org", ".net", ".io", ".tech", ".cn", ".co", ".me"];
        
        let name = names[fastrand::usize(0..names.len())];
        let tld = tlds[fastrand::usize(0..tlds.len())];
        
        format!("{}{}", name, tld)
    }
    
    fn generate_protocol() -> String {
        let protocols = ["http", "https", "ftp", "ssh", "sftp", "ldap", "smtp", "pop3", "imap"];
        protocols[fastrand::usize(0..protocols.len())].to_string()
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
    
    fn generate_url() -> String {
        let protocol = ["http", "https"][fastrand::usize(0..2)];
        let domain = Self::generate_domain();
        let paths = ["", "/api", "/user", "/products", "/services", "/about", "/contact"];
        let path = paths[fastrand::usize(0..paths.len())];
        
        if path.is_empty() {
            format!("{}://{}", protocol, domain)
        } else {
            format!("{}://{}{}", protocol, domain, path)
        }
    }
    
    fn generate_mac() -> String {
        format!("{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            fastrand::u8(0..255),
            fastrand::u8(0..255),
            fastrand::u8(0..255),
            fastrand::u8(0..255),
            fastrand::u8(0..255),
            fastrand::u8(0..255))
    }
}
