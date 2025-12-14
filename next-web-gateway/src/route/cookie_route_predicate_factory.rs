use super::route_predicate::RoutePredicate;

#[derive(Debug, Clone)]
pub struct CookieRoutePredicateFactory {
    pub cookies: Vec<String>,
}

impl RoutePredicate for CookieRoutePredicateFactory {
    fn matches(&self, session: &mut pingora::protocols::http::ServerSession) -> bool {
        // 获取 Cookie 头，假设 get 是大小写不敏感的
        if let Some(cookie_header) = session.req_header().headers.get("Cookie") {
            // 确保 header 不为空
            if !cookie_header.is_empty() {
                // 尝试转换为字符串
                if let Ok(cookie_str) = cookie_header.to_str() {
                    // 对整个 cookie 字符串 trim_end，然后按分号分割
                    return cookie_str
                        .trim_end() // 去除末尾空格/换行
                        .split(';') // 直接使用 Iterator，避免 collect
                        .any(|cookie| {
                            // 对每个 cookie 片段进行 trim，去除前后空格
                            let trimmed_cookie = cookie.trim();
                            // 检查是否以 self.cookies 中的任意一个前缀开头
                            self.cookies
                                .iter()
                                .any(|prefix| trimmed_cookie.starts_with(prefix))
                        });
                }
            }
        }
        false
    }
}
