use std::fmt;

#[derive(Debug, PartialEq, Clone)]
pub struct UserAgentInfo {
    pub browser: Option<String>,
    pub browser_version: Option<String>,
    pub os: Option<String>,
    pub os_version: Option<String>,
    pub device: Option<String>,
    pub engine: Option<String>,
    pub engine_version: Option<String>,
}

impl UserAgentInfo {
    pub fn parse(ua: &str) -> Self {
        let mut info = UserAgentInfo {
            browser: None,
            browser_version: None,
            os: None,
            os_version: None,
            device: None,
            engine: None,
            engine_version: None,
        };

        let ua_lower = ua.to_lowercase();

        // 浏览器识别
        Self::extract_browser(&mut info, &ua_lower);

        // 操作系统识别
        Self::extract_os(&mut info, &ua_lower);

        // 设备识别
        Self::extract_device(&mut info, &ua_lower);

        // 渲染引擎识别
        Self::extract_engine(&mut info, &ua_lower);

        // 后处理
        Self::post_process(&mut info);

        info
    }

    fn extract_browser(info: &mut UserAgentInfo, ua_lower: &str) {
        let browsers = [
            "firefox", "chrome", "safari", "opera", "msie", "trident", "edge", "edg",
            "netscape", "maxthon", "konqueror", "lynx", "ucbrowser",
        ];

        for browser in browsers {
            if let Some(pos) = ua_lower.find(browser) {
                info.browser = Some(browser.to_string());
                info.browser_version = Self::extract_version(ua_lower, pos + browser.len());
                break;
            }
        }

        // 如果没找到浏览器信息，尝试其他模式
        if info.browser.is_none() {
            let bots = [
                "facebookexternalhit",
                "twitterbot",
                "googlebot",
                "bingbot",
                "yandexbot",
                "slurp",
                "duckduckbot",
                "baiduspider",
            ];

            for bot in bots {
                if ua_lower.contains(bot) {
                    info.browser = Some(bot.to_string());
                    break;
                }
            }
        }
    }

    fn extract_os(info: &mut UserAgentInfo, ua_lower: &str) {
        let os_list = [
            ("windows nt", "windows nt"),
            ("windows", "windows"),
            ("mac os x", "mac os x"),
            ("macintosh", "macintosh"),
            ("linux", "linux"),
            ("ubuntu", "ubuntu"),
            ("android", "android"),
            ("iphone os", "iphone os"),
            ("ios", "ios"),
            ("blackberry", "blackberry"),
            ("symbianos", "symbianos"),
            ("webos", "webos"),
        ];

        for (os_key, os_name) in os_list {
            if let Some(pos) = ua_lower.find(os_key) {
                info.os = Some(os_name.to_string());
                info.os_version = Self::extract_version(ua_lower, pos + os_key.len());
                break;
            }
        }
    }

    fn extract_device(info: &mut UserAgentInfo, ua_lower: &str) {
        let devices = [
            "iphone", "ipad", "ipod", "blackberry", "htc", "samsung", "nokia", "nexus",
            "kindle", "playbook", "xbox", "playstation", "smart-tv",
        ];

        for device in devices {
            if ua_lower.contains(device) {
                info.device = Some(device.to_string());
                break;
            }
        }
    }

    fn extract_engine(info: &mut UserAgentInfo, ua_lower: &str) {
        let engines = [
            "webkit", "gecko", "trident", "presto", "blink", "khtml"
        ];

        for engine in engines {
            if let Some(pos) = ua_lower.find(engine) {
                info.engine = Some(engine.to_string());
                info.engine_version = Self::extract_version(ua_lower, pos + engine.len());
                break;
            }
        }
    }

    fn extract_version(ua_lower: &str, start_pos: usize) -> Option<String> {
        let remaining = &ua_lower[start_pos..];
        
        // 查找版本号通常跟在 '/' 或 ' ' 后面
        let version_start = remaining.find(|c: char| c == '/' || c == ' ' || c == ';')? + 1;
        let version_end = remaining[version_start..]
            .find(|c: char| !(c.is_ascii_digit() || c == '.'))
            .unwrap_or(remaining.len() - version_start);
        
        let version_str = &remaining[version_start..version_start + version_end];
        if !version_str.is_empty() {
            Some(version_str.to_string())
        } else {
            None
        }
    }

    fn post_process(info: &mut UserAgentInfo) {
        if let Some(browser) = &info.browser {
            // 处理IE/Trident的特殊情况
            if browser == "trident" {
                info.browser = Some("internet explorer".to_string());
                if let Some(version) = &info.browser_version {
                    // Trident版本到IE版本的映射
                    let ie_version = match version.as_str() {
                        "7.0" => "11.0",
                        "6.0" => "10.0",
                        "5.0" => "9.0",
                        "4.0" => "8.0",
                        _ => version.as_str(),
                    };
                    info.browser_version = Some(ie_version.to_string());
                }
            } else if browser == "edg" {
                info.browser = Some("microsoft edge".to_string());
            }
        }
    }
}

impl fmt::Display for UserAgentInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Browser: {}\nVersion: {}\nOS: {}\nOS Version: {}\nDevice: {}\nEngine: {}\nEngine Version: {}",
            self.browser.as_deref().unwrap_or("Unknown"),
            self.browser_version.as_deref().unwrap_or("Unknown"),
            self.os.as_deref().unwrap_or("Unknown"),
            self.os_version.as_deref().unwrap_or("Unknown"),
            self.device.as_deref().unwrap_or("Unknown"),
            self.engine.as_deref().unwrap_or("Unknown"),
            self.engine_version.as_deref().unwrap_or("Unknown"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_user_agent() {
        let ua = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36";
        let info = UserAgentInfo::parse(ua);
        
        assert_eq!(info.browser, Some("chrome".to_string()));
        assert_eq!(info.browser_version, Some("91.0.4472.124".to_string()));
        assert_eq!(info.os, Some("windows nt".to_string()));
        assert_eq!(info.os_version, Some("10.0".to_string()));
        assert_eq!(info.engine, Some("webkit".to_string()));
        assert_eq!(info.engine_version, Some("537.36".to_string()));
    }

    #[test]
    fn test_parse_internet_explorer() {
        let ua = "Mozilla/5.0 (Windows NT 6.1; WOW64; Trident/7.0; rv:11.0) like Gecko";
        let info = UserAgentInfo::parse(ua);
        
        assert_eq!(info.browser, Some("internet explorer".to_string()));
        assert_eq!(info.browser_version, Some("11.0".to_string()));
        assert_eq!(info.os, Some("windows nt".to_string()));
        assert_eq!(info.os_version, Some("6.1".to_string()));
    }

    #[test]
    fn test_parse_safari() {
        let ua = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/14.1.2 Safari/605.1.15";
        let info = UserAgentInfo::parse(ua);
        
        assert_eq!(info.browser, Some("safari".to_string()));
        assert_eq!(info.browser_version, Some("605.1.15".to_string()));
        assert_eq!(info.os, Some("mac os x".to_string()));
        assert_eq!(info.engine, Some("webkit".to_string()));
        assert_eq!(info.engine_version, Some("605.1.15".to_string()));
    }
}