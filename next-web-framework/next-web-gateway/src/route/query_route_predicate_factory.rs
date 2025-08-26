use super::route_predicate::RoutePredicate;

#[derive(Debug, Clone)]
pub struct QueryRoutePredicateFactory {
    pub name: String,
}

impl RoutePredicate for QueryRoutePredicateFactory {
    fn matches(&self, session: &mut pingora::protocols::http::ServerSession) -> bool {
        // 1. 获取查询字符串
        let query_str = match session.req_header().uri.query() {
            Some(q) => q,
            None => return false, // 没有查询字符串，不匹配
        };

        // 2. 按 '&' 分割并直接使用迭代器检查
        query_str.split('&').any(|param| {
            // 检查参数是否以 `name` 开头
            // 这涵盖了 `name` 和 `name=value` 两种情况
            param == self.name || param.starts_with(&format!("{}=", &self.name))
        })
    }
}
