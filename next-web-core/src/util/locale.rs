use std::fmt;
use std::str::FromStr;

use sys_locale::get_locale;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Locale {
    ZhCn,
    ZhTw,
    ZhHk,
    EnHk,
    #[default]
    EnUs,
    EnGb,
    EnWw,
    EnCa,
    EnAu,
    EnIe,
    EnFi,
    EnDk,
    EnIl,
    EnZa,
    EnIn,
    EnNo,
    EnSg,
    EnNz,
    EnId,
    EnPh,
    EnTh,
    EnMy,
    EnXa,
    KoKr,
    JaJp,
    NlNl,
    NlBe,
    PtPt,
    PtBr,
    FrFr,
    FrLu,
    FrCh,
    FrBe,
    FrCa,
    EsLa,
    EsEs,
    EsAr,
    EsUs,
    EsMx,
    EsCo,
    EsPr,
    DeDe,
    DeAt,
    DeCh,
    RuRu,
    ItIt,
    ElGr,
    NoNo,
    HuHu,
    TrTr,
    CsCz,
    SlSl,
    PlPl,
    SvSe,
    FiFi,
    DaDk,
    HeIl,
}

const ALL_LOCALES: &[Locale] = &[
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
];

impl Locale {
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
            Locale::SlSl => "sl-SI",
            Locale::PlPl => "pl-PL",
            Locale::SvSe => "sv-SE",
            Locale::FiFi => "fi-FI",
            Locale::DaDk => "da-DK",
            Locale::HeIl => "he-IL",
        }
    }

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
            Locale::EnXa => "英语(伪本地化)",
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
            Locale::SlSl => "斯洛文尼亚语(斯洛文尼亚)",
            Locale::PlPl => "波兰语(波兰)",
            Locale::SvSe => "瑞典语(瑞典)",
            Locale::FiFi => "芬兰语(芬兰)",
            Locale::DaDk => "丹麦语(丹麦)",
            Locale::HeIl => "希伯来语(以色列)",
        }
    }

    pub fn locale() -> Self {
        get_locale()
            .as_deref()
            .and_then(Self::match_locale_tag)
            .unwrap_or(Locale::EnUs)
    }

    pub fn from_accept_language(language: impl AsRef<str>) -> Option<Locale> {
        let language = language.as_ref().trim();
        if language.is_empty() {
            return None;
        }

        let mut preferences: Vec<(usize, String, f32)> = Vec::new();

        for (index, part) in language.split(',').enumerate() {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }

            let mut q = 1.0_f32;
            let mut tag = part;

            if let Some(sep_idx) = part.find(';') {
                tag = part[..sep_idx].trim();

                for option in part[sep_idx + 1..].split(';') {
                    let option = option.trim();
                    let Some((name, value)) = option.split_once('=') else {
                        continue;
                    };

                    if name.trim().eq_ignore_ascii_case("q") {
                        if let Ok(parsed) = value.trim().parse::<f32>() {
                            if (0.0..=1.0).contains(&parsed) {
                                q = parsed;
                            }
                        }
                    }
                }
            }

            if !tag.is_empty() && q > 0.0 {
                preferences.push((index, tag.to_string(), q));
            }
        }

        if preferences.is_empty() {
            return None;
        }

        preferences.sort_by(|a, b| {
            b.2.partial_cmp(&a.2)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });

        for (_, tag, _) in preferences {
            if tag == "*" {
                return Some(Locale::default());
            }

            if let Some(locale) = Self::match_locale_tag(&tag) {
                return Some(locale);
            }
        }

        None
    }

    pub fn all_locales() -> Vec<Locale> {
        ALL_LOCALES.to_vec()
    }

    fn match_locale_tag(tag: &str) -> Option<Locale> {
        let normalized = normalize_locale_tag(tag);
        if normalized.is_empty() {
            return None;
        }

        if let Ok(locale) = Locale::from_str(&normalized) {
            return Some(locale);
        }

        let mut parts = normalized.split('-').collect::<Vec<_>>();
        while parts.len() > 1 {
            parts.pop();
            let candidate = parts.join("-");
            if let Ok(locale) = Locale::from_str(&candidate) {
                return Some(locale);
            }
        }

        primary_language_locale(parts.first().copied().unwrap_or_default())
    }
}

impl FromStr for Locale {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match normalize_locale_tag(s).as_str() {
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
            "zh" => Ok(Locale::ZhCn),
            "en" => Ok(Locale::EnUs),
            "ko" => Ok(Locale::KoKr),
            "ja" => Ok(Locale::JaJp),
            "nl" => Ok(Locale::NlNl),
            "pt" => Ok(Locale::PtPt),
            "fr" => Ok(Locale::FrFr),
            "es" => Ok(Locale::EsEs),
            "de" => Ok(Locale::DeDe),
            "ru" => Ok(Locale::RuRu),
            "it" => Ok(Locale::ItIt),
            "el" => Ok(Locale::ElGr),
            "no" => Ok(Locale::NoNo),
            "hu" => Ok(Locale::HuHu),
            "tr" => Ok(Locale::TrTr),
            "cs" => Ok(Locale::CsCz),
            "sl" => Ok(Locale::SlSl),
            "pl" => Ok(Locale::PlPl),
            "sv" => Ok(Locale::SvSe),
            "fi" => Ok(Locale::FiFi),
            "da" => Ok(Locale::DaDk),
            "he" => Ok(Locale::HeIl),
            _ => Err("Invalid locale string"),
        }
    }
}

impl fmt::Display for Locale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

fn normalize_locale_tag(input: &str) -> String {
    let trimmed = input.trim();
    let without_encoding = trimmed
        .split_once('.')
        .map(|(head, _)| head)
        .unwrap_or(trimmed);
    let without_modifier = without_encoding
        .split_once('@')
        .map(|(head, _)| head)
        .unwrap_or(without_encoding);

    without_modifier.replace('_', "-").to_ascii_lowercase()
}

fn primary_language_locale(language: &str) -> Option<Locale> {
    match language {
        "zh" => Some(Locale::ZhCn),
        "en" => Some(Locale::EnUs),
        "ko" => Some(Locale::KoKr),
        "ja" => Some(Locale::JaJp),
        "nl" => Some(Locale::NlNl),
        "pt" => Some(Locale::PtPt),
        "fr" => Some(Locale::FrFr),
        "es" => Some(Locale::EsEs),
        "de" => Some(Locale::DeDe),
        "ru" => Some(Locale::RuRu),
        "it" => Some(Locale::ItIt),
        "el" => Some(Locale::ElGr),
        "no" => Some(Locale::NoNo),
        "hu" => Some(Locale::HuHu),
        "tr" => Some(Locale::TrTr),
        "cs" => Some(Locale::CsCz),
        "sl" => Some(Locale::SlSl),
        "pl" => Some(Locale::PlPl),
        "sv" => Some(Locale::SvSe),
        "fi" => Some(Locale::FiFi),
        "da" => Some(Locale::DaDk),
        "he" => Some(Locale::HeIl),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::Locale;
    use std::str::FromStr;

    #[test]
    fn from_str_accepts_standard_and_alias_tags() {
        assert_eq!(Locale::from_str("en-US").ok(), Some(Locale::EnUs));
        assert_eq!(Locale::from_str("sl-SI").ok(), Some(Locale::SlSl));
        assert_eq!(Locale::from_str("en_US.UTF-8").ok(), Some(Locale::EnUs));
        assert_eq!(Locale::from_str("zh").ok(), Some(Locale::ZhCn));
    }

    #[test]
    fn as_str_returns_standardized_bcp47_tag() {
        assert_eq!(Locale::SlSl.as_str(), "sl-SI");
    }

    #[test]
    fn from_language_prefers_highest_quality_exact_match() {
        let header = "zh-CN,zh;q=0.9,en;q=0.8,en-GB;q=0.7,en-US;q=0.6";
        assert_eq!(Locale::from_accept_language(header), Some(Locale::ZhCn));
    }

    #[test]
    fn from_language_falls_back_to_primary_language() {
        assert_eq!(Locale::from_accept_language("en;q=0.9"), Some(Locale::EnUs));
        assert_eq!(Locale::from_accept_language("zh;q=0.9"), Some(Locale::ZhCn));
    }

    #[test]
    fn from_language_can_reduce_unknown_subtags() {
        assert_eq!(
            Locale::from_accept_language("fr-CA-x-private;q=0.9"),
            Some(Locale::FrCa)
        );
        assert_eq!(
            Locale::from_accept_language("zh-Hans-CN;q=0.9"),
            Some(Locale::ZhCn)
        );
    }

    #[test]
    fn from_language_supports_wildcard() {
        assert_eq!(Locale::from_accept_language("*;q=0.5"), Some(Locale::EnUs));
    }

    #[test]
    fn from_language_ignores_zero_quality_values() {
        assert_eq!(
            Locale::from_accept_language("zh-CN;q=0,en-US;q=0.8"),
            Some(Locale::EnUs)
        );
        assert_eq!(Locale::from_accept_language("zh-CN;q=0,*;q=0"), None);
    }

    #[test]
    fn from_language_accepts_case_insensitive_q_parameter() {
        assert_eq!(
            Locale::from_accept_language("fr-CA;Q=0.9,en-US;q=0.8"),
            Some(Locale::FrCa)
        );
    }
}
