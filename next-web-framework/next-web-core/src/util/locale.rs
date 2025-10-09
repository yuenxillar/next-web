use std::str::FromStr;

use sys_locale::get_locale;

/// 表示不同的语言环境 (Locale)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Locale {
    // --- 中文 ---
    /// 简体中文 (中国)
    ZhCn,
    /// 繁体中文 (台湾地区)
    ZhTw,
    /// 繁体中文 (香港)
    ZhHk,

    // --- 英语 ---
    /// 英语 (香港)
    EnHk,
    /// 英语 (美国)
    EnUs,
    /// 英语 (英国)
    EnGb,
    /// 英语 (全球)
    EnWw,
    /// 英语 (加拿大)
    EnCa,
    /// 英语 (澳大利亚)
    EnAu,
    /// 英语 (爱尔兰)
    EnIe,
    /// 英语 (芬兰)
    EnFi,
    /// 英语 (丹麦)
    EnDk,
    /// 英语 (以色列)
    EnIl,
    /// 英语 (南非)
    EnZa,
    /// 英语 (印度)
    EnIn,
    /// 英语 (挪威)
    EnNo,
    /// 英语 (新加坡)
    EnSg,
    /// 英语 (新西兰)
    EnNz,
    /// 英语 (印度尼西亚)
    EnId,
    /// 英语 (菲律宾)
    EnPh,
    /// 英语 (泰国)
    EnTh,
    /// 英语 (马来西亚)
    EnMy,
    /// 英语 (阿拉伯)
    EnXa,

    // --- 其他语言 ---
    /// 韩文 (韩国)
    KoKr,
    /// 日语 (日本)
    JaJp,
    /// 荷兰语 (荷兰)
    NlNl,
    /// 荷兰语 (比利时)
    NlBe,
    /// 葡萄牙语 (葡萄牙)
    PtPt,
    /// 葡萄牙语 (巴西)
    PtBr,
    /// 法语 (法国)
    FrFr,
    /// 法语 (卢森堡)
    FrLu,
    /// 法语 (瑞士)
    FrCh,
    /// 法语 (比利时)
    FrBe,
    /// 法语 (加拿大)
    FrCa,
    /// 西班牙语 (拉丁美洲)
    EsLa,
    /// 西班牙语 (西班牙)
    EsEs,
    /// 西班牙语 (阿根廷)
    EsAr,
    /// 西班牙语 (美国)
    EsUs,
    /// 西班牙语 (墨西哥)
    EsMx,
    /// 西班牙语 (哥伦比亚)
    EsCo,
    /// 西班牙语 (波多黎各)
    EsPr,
    /// 德语 (德国)
    DeDe,
    /// 德语 (奥地利)
    DeAt,
    /// 德语 (瑞士)
    DeCh,
    /// 俄语 (俄罗斯)
    RuRu,
    /// 意大利语 (意大利)
    ItIt,
    /// 希腊语 (希腊)
    ElGr,
    /// 挪威语 (挪威)
    NoNo,
    /// 匈牙利语 (匈牙利)
    HuHu,
    /// 土耳其语 (土耳其)
    TrTr,
    /// 捷克语 (捷克共和国)
    CsCz,
    /// 斯洛文尼亚语
    SlSl,
    /// 波兰语 (波兰)
    PlPl,
    /// 瑞典语 (瑞典)
    SvSe,
    /// 芬兰语 (芬兰)
    FiFi,
    /// 丹麦语 (丹麦)
    DaDk,
    /// 希伯来语 (以色列)
    HeIl,
}

impl Locale {
    /// 获取此语言环境的 BCP 47 语言标签 (如 "zh-CN", "en-US")
    pub fn as_str(&self) -> &'static str {
        match self {
            Locale::ZhCn => "zh-CN",
            Locale::ZhTw => "zh-TW",
            Locale::ZhHk => "zh-HK",
            Locale::EnHk => "en-HK",
            Locale::EnUs => "en-US",
            Locale::EnGb => "en-GB",
            Locale::EnWw => "en-WW",
            Locale::EnCa => "en-CA",
            Locale::EnAu => "en-AU",
            Locale::EnIe => "en-IE",
            Locale::EnFi => "en-FI",
            Locale::EnDk => "en-DK",
            Locale::EnIl => "en-IL",
            Locale::EnZa => "en-ZA",
            Locale::EnIn => "en-IN",
            Locale::EnNo => "en-NO",
            Locale::EnSg => "en-SG",
            Locale::EnNz => "en-NZ",
            Locale::EnId => "en-ID",
            Locale::EnPh => "en-PH",
            Locale::EnTh => "en-TH",
            Locale::EnMy => "en-MY",
            Locale::EnXa => "en-XA",
            Locale::KoKr => "ko-KR",
            Locale::JaJp => "ja-JP",
            Locale::NlNl => "nl-NL",
            Locale::NlBe => "nl-BE",
            Locale::PtPt => "pt-PT",
            Locale::PtBr => "pt-BR",
            Locale::FrFr => "fr-FR",
            Locale::FrLu => "fr-LU",
            Locale::FrCh => "fr-CH",
            Locale::FrBe => "fr-BE",
            Locale::FrCa => "fr-CA",
            Locale::EsLa => "es-LA",
            Locale::EsEs => "es-ES",
            Locale::EsAr => "es-AR",
            Locale::EsUs => "es-US",
            Locale::EsMx => "es-MX",
            Locale::EsCo => "es-CO",
            Locale::EsPr => "es-PR",
            Locale::DeDe => "de-DE",
            Locale::DeAt => "de-AT",
            Locale::DeCh => "de-CH",
            Locale::RuRu => "ru-RU",
            Locale::ItIt => "it-IT",
            Locale::ElGr => "el-GR",
            Locale::NoNo => "no-NO",
            Locale::HuHu => "hu-HU",
            Locale::TrTr => "tr-TR",
            Locale::CsCz => "cs-CZ",
            Locale::SlSl => "sl-SL", // 注意: 标准 BCP 47 是 sl-SI (斯洛文尼亚)
            Locale::PlPl => "pl-PL",
            Locale::SvSe => "sv-SE",
            Locale::FiFi => "fi-FI",
            Locale::DaDk => "da-DK",
            Locale::HeIl => "he-IL",
        }
    }

    /// 获取此语言环境的友好显示名称
    pub fn display_name(&self) -> &'static str {
        match self {
            Locale::ZhCn => "简体中文(中国)",
            Locale::ZhTw => "繁体中文(台湾地区)",
            Locale::ZhHk => "繁体中文(香港)",
            Locale::EnHk => "英语(香港)",
            Locale::EnUs => "英语(美国)",
            Locale::EnGb => "英语(英国)",
            Locale::EnWw => "英语(全球)",
            Locale::EnCa => "英语(加拿大)",
            Locale::EnAu => "英语(澳大利亚)",
            Locale::EnIe => "英语(爱尔兰)",
            Locale::EnFi => "英语(芬兰)",
            Locale::EnDk => "英语(丹麦)",
            Locale::EnIl => "英语(以色列)",
            Locale::EnZa => "英语(南非)",
            Locale::EnIn => "英语(印度)",
            Locale::EnNo => "英语(挪威)",
            Locale::EnSg => "英语(新加坡)",
            Locale::EnNz => "英语(新西兰)",
            Locale::EnId => "英语(印度尼西亚)",
            Locale::EnPh => "英语(菲律宾)",
            Locale::EnTh => "英语(泰国)",
            Locale::EnMy => "英语(马来西亚)",
            Locale::EnXa => "英语(阿拉伯)",
            Locale::KoKr => "韩文(韩国)",
            Locale::JaJp => "日语(日本)",
            Locale::NlNl => "荷兰语(荷兰)",
            Locale::NlBe => "荷兰语(比利时)",
            Locale::PtPt => "葡萄牙语(葡萄牙)",
            Locale::PtBr => "葡萄牙语(巴西)",
            Locale::FrFr => "法语(法国)",
            Locale::FrLu => "法语(卢森堡)",
            Locale::FrCh => "法语(瑞士)",
            Locale::FrBe => "法语(比利时)",
            Locale::FrCa => "法语(加拿大)",
            Locale::EsLa => "西班牙语(拉丁美洲)",
            Locale::EsEs => "西班牙语(西班牙)",
            Locale::EsAr => "西班牙语(阿根廷)",
            Locale::EsUs => "西班牙语(美国)",
            Locale::EsMx => "西班牙语(墨西哥)",
            Locale::EsCo => "西班牙语(哥伦比亚)",
            Locale::EsPr => "西班牙语(波多黎各)",
            Locale::DeDe => "德语(德国)",
            Locale::DeAt => "德语(奥地利)",
            Locale::DeCh => "德语(瑞士)",
            Locale::RuRu => "俄语(俄罗斯)",
            Locale::ItIt => "意大利语(意大利)",
            Locale::ElGr => "希腊语(希腊)",
            Locale::NoNo => "挪威语(挪威)",
            Locale::HuHu => "匈牙利语(匈牙利)",
            Locale::TrTr => "土耳其语(土耳其)",
            Locale::CsCz => "捷克语(捷克共和国)",
            Locale::SlSl => "斯洛文尼亚语",
            Locale::PlPl => "波兰语(波兰)",
            Locale::SvSe => "瑞典语(瑞典)",
            Locale::FiFi => "芬兰语(芬兰)",
            Locale::DaDk => "丹麦语(丹麦)",
            Locale::HeIl => "希伯来语(以色列)",
        }
    }
}

// 为 Locale 实现 FromStr trait，方便从字符串解析
impl std::str::FromStr for Locale {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim().replace("_", "-");
        match s.to_lowercase().as_str() {
            "zh-cn" => Ok(Locale::ZhCn),
            "zh-tw" => Ok(Locale::ZhTw),
            "zh-hk" => Ok(Locale::ZhHk),
            "en-hk" => Ok(Locale::EnHk),
            "en-us" => Ok(Locale::EnUs),
            "en-gb" => Ok(Locale::EnGb),
            "en-ww" => Ok(Locale::EnWw),
            "en-ca" => Ok(Locale::EnCa),
            "en-au" => Ok(Locale::EnAu),
            "en-ie" => Ok(Locale::EnIe),
            "en-fi" => Ok(Locale::EnFi),
            "en-dk" => Ok(Locale::EnDk),
            "en-il" => Ok(Locale::EnIl),
            "en-za" => Ok(Locale::EnZa),
            "en-in" => Ok(Locale::EnIn),
            "en-no" => Ok(Locale::EnNo),
            "en-sg" => Ok(Locale::EnSg),
            "en-nz" => Ok(Locale::EnNz),
            "en-id" => Ok(Locale::EnId),
            "en-ph" => Ok(Locale::EnPh),
            "en-th" => Ok(Locale::EnTh),
            "en-my" => Ok(Locale::EnMy),
            "en-xa" => Ok(Locale::EnXa),
            "ko-kr" => Ok(Locale::KoKr),
            "ja-jp" => Ok(Locale::JaJp),
            "nl-nl" => Ok(Locale::NlNl),
            "nl-be" => Ok(Locale::NlBe),
            "pt-pt" => Ok(Locale::PtPt),
            "pt-br" => Ok(Locale::PtBr),
            "fr-fr" => Ok(Locale::FrFr),
            "fr-lu" => Ok(Locale::FrLu),
            "fr-ch" => Ok(Locale::FrCh),
            "fr-be" => Ok(Locale::FrBe),
            "fr-ca" => Ok(Locale::FrCa),
            "es-la" => Ok(Locale::EsLa),
            "es-es" => Ok(Locale::EsEs),
            "es-ar" => Ok(Locale::EsAr),
            "es-us" => Ok(Locale::EsUs),
            "es-mx" => Ok(Locale::EsMx),
            "es-co" => Ok(Locale::EsCo),
            "es-pr" => Ok(Locale::EsPr),
            "de-de" => Ok(Locale::DeDe),
            "de-at" => Ok(Locale::DeAt),
            "de-ch" => Ok(Locale::DeCh),
            "ru-ru" => Ok(Locale::RuRu),
            "it-it" => Ok(Locale::ItIt),
            "el-gr" => Ok(Locale::ElGr),
            "no-no" => Ok(Locale::NoNo),
            "hu-hu" => Ok(Locale::HuHu),
            "tr-tr" => Ok(Locale::TrTr),
            "cs-cz" => Ok(Locale::CsCz),
            "sl-sl" | "sl-si" => Ok(Locale::SlSl),
            "pl-pl" => Ok(Locale::PlPl),
            "sv-se" => Ok(Locale::SvSe),
            "fi-fi" => Ok(Locale::FiFi),
            "da-dk" => Ok(Locale::DaDk),
            "he-il" => Ok(Locale::HeIl),
            _ => Err("Invalid locale string"),
        }
    }
}

impl Locale {
    pub fn locale() -> Self {
        Locale::from_str(&get_locale().unwrap_or("en-us".into())).unwrap_or(Locale::EnUs)
    }

    /// 根据 Accept-Language 请求头字符串解析并返回最匹配的受支持的 Locale。
    ///
    /// 该函数解析类似 "zh-CN,zh;q=0.9,en;q=0.8,en-GB;q=0.7,en-US;q=0.6" 的字符串。
    /// 它会：
    /// 1. 按逗号分割。
    /// 2. 解析每个项的语言标签和 q 值（默认 q=1.0）。
    /// 3. 按 q 值降序排序。
    /// 4. 遍历排序后的列表，返回第一个能成功匹配到 `Locale` 枚举的项。
    ///
    /// # 参数
    /// * `language`: 实现 `AsRef<str>` 的类型，通常是 &str 或 String，表示 Accept-Language 头。
    ///
    /// # 返回值
    /// * `Some(Locale)`: 找到了匹配的受支持语言环境。
    /// * `None`: 输入为空、格式错误或没有找到受支持的语言。
    ///
    /// # 示例
    /// ```
    /// use your_crate::Locale; // 假设枚举在 your_crate 模块中
    ///
    /// let header = "zh-CN,zh;q=0.9,en;q=0.8,en-GB;q=0.7,en-US;q=0.6";
    /// let locale = Locale::from_language(header);
    /// assert_eq!(locale, Some(Locale::ZhCn));
    ///
    /// let header_en = "en-GB,en;q=0.8,en-US;q=0.6";
    /// let locale = Locale::from_language(header_en);
    /// assert_eq!(locale, Some(Locale::EnGb)); // en-GB 存在
    ///
    /// let header_only_unsupported = "fr-FR;q=0.9,de-DE;q=0.8";
    /// let locale = Locale::from_language(header_only_unsupported);
    /// assert_eq!(locale, None); // 没有受支持的匹配项
    /// ```
    pub fn from_language(language: impl AsRef<str>) -> Option<Locale> {
        // TODO
        let language = language.as_ref().trim();

        // 如果字符串为空，直接返回 None
        if language.is_empty() {
            return None;
        }

        // 用于存储解析后的语言偏好及其质量值
        let mut preferences: Vec<(String, f32)> = Vec::new();

        // 1. 按逗号分割字符串
        for part in language.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue; // 跳过空部分
            }

            // 2. 解析每个部分：可能包含 q 值
            let mut q = 1.0; // 默认质量值
            let mut tag = part;

            // 查找分号 ';'
            if let Some(sep_idx) = part.find(';') {
                let before_semi = &part[..sep_idx].trim();
                let after_semi = &part[sep_idx + 1..].trim();

                tag = before_semi;

                // 解析 q 值
                if after_semi.starts_with("q=") {
                    if let Ok(q_val) = after_semi[2..].parse::<f32>() {
                        // 确保 q 值在 0.0 到 1.0 之间，无效的 q 值通常被忽略或视为 0
                        if (0.0..=1.0).contains(&q_val) {
                            q = q_val;
                        }
                        // 如果 q 值无效，保持默认 q=1.0 或根据规范处理（这里选择忽略无效 q）
                    }
                    // 如果 q= 后面不是数字，也保持默认 q=1.0
                }
                // 如果分号后不是 q=...，也保持默认 q=1.0 (虽然不标准，但可以容忍)
            }

            // 只有非空标签才添加
            if !tag.is_empty() {
                preferences.push((tag.to_lowercase(), q)); // 转为小写便于后续比较
            }
        }

        // 3. 如果没有解析到任何有效偏好，返回 None
        if preferences.is_empty() {
            return None;
        }

        // 4. 按质量值 q 降序排序 (q 值高的优先)
        // 使用 stable_sort 以保持相同 q 值项的原始顺序
        preferences.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // 5. 遍历排序后的偏好列表
        for (tag, _q) in &preferences {
            // 尝试将语言标签解析为 Locale 枚举
            // 我们的 FromStr 实现已经处理了大小写和特定标签
            if let Ok(locale) = tag.parse::<Locale>() {
                return Some(locale); // 找到第一个匹配项即返回
            }
            // 注意：这里没有实现更复杂的匹配逻辑（如 zh-CN 匹配 zh），因为
            // 我们的 Locale 枚举是具体的。如果需要这种逻辑，需要更复杂的匹配函数。
            // TODO: 实现更复杂的匹配逻辑
        }

        // 6. 如果遍历完都没有找到匹配的 Locale，返回 None
        None
    }
}

impl Locale {
    pub fn all_locales() -> Vec<Locale> {
        vec![
            Locale::ZhCn,
            Locale::ZhTw,
            Locale::ZhHk,
            Locale::EnHk,
            Locale::EnUs,
            Locale::EnGb,
            Locale::EnWw,
            Locale::EnCa,
            Locale::EnAu,
            Locale::EnIe,
            Locale::EnFi,
            Locale::EnDk,
            Locale::EnIl,
            Locale::EnZa,
            Locale::EnIn,
            Locale::EnNo,
            Locale::EnSg,
            Locale::EnNz,
            Locale::EnId,
            Locale::EnPh,
            Locale::EnTh,
            Locale::EnMy,
            Locale::EnXa,
            Locale::KoKr,
            Locale::JaJp,
            Locale::NlNl,
            Locale::NlBe,
            Locale::PtPt,
            Locale::PtBr,
            Locale::FrFr,
            Locale::FrLu,
            Locale::FrCh,
            Locale::FrBe,
            Locale::FrCa,
            Locale::EsLa,
            Locale::EsEs,
            Locale::EsAr,
            Locale::EsUs,
            Locale::EsMx,
            Locale::EsCo,
            Locale::EsPr,
            Locale::DeDe,
            Locale::DeAt,
            Locale::DeCh,
            Locale::RuRu,
            Locale::ItIt,
            Locale::ElGr,
            Locale::NoNo,
            Locale::HuHu,
            Locale::TrTr,
            Locale::CsCz,
            Locale::SlSl,
            Locale::PlPl,
            Locale::SvSe,
            Locale::FiFi,
            Locale::DaDk,
            Locale::HeIl,
        ]
    }
}
