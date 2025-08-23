use crate::util::key_value::KeyValue;
use super::route_predicate::RoutePredicate;

#[derive(Debug, Clone)]
pub struct HeaderRoutePredicateFactory {
    pub header: KeyValue<Option<String>>,
    pub regex: Option<regex::Regex>,
}

impl RoutePredicate for HeaderRoutePredicateFactory {
    fn matches(&self, session: &mut pingora::protocols::http::ServerSession) -> bool {
        let header_name = self.header.k.as_ref();
        
        // 1. 检查头是否存在
        if let Some(header_value_bytes) = session.req_header().headers.get(header_name) {
            // 2. 尝试将 header 值转换为字符串
            //    使用 `from_utf8_lossy` 比 `to_str().unwrap_or("")` 更安全，能处理部分非 UTF-8
            let header_value_str = String::from_utf8_lossy(header_value_bytes.as_bytes());

            // 3. 核心逻辑：根据 self.header.v 的值决定匹配策略
            match &self.header.v {
                // v 为 None：只要头存在就匹配成功
                None => {
                    return true;
                }
                // v 为 Some(_)：需要进一步检查
                Some(_) => {
                    // 如果提供了正则表达式，则使用正则匹配
                    if let Some(ref regex) = self.regex {
                        return regex.is_match(&header_value_str);
                    }
                    return false;
                }
            }
        }

        false
    }
}