use super::route_predicate::RoutePredicate;
use matchit::Router;

#[derive(Debug, Clone)]
pub struct PathRoutePredicateFactory {
    pub paths: Router<bool>,
}

impl RoutePredicate for PathRoutePredicateFactory {
    fn matches(&self, session: &mut pingora::protocols::http::ServerSession) -> bool {
        let raw_path = session.req_header().raw_path();
        
        // 1. 使用 `from_utf8_lossy` 安全转换，但直接传递 `&str`
        let path_str = match std::str::from_utf8(raw_path) {
            Ok(s) => s,
            Err(_) => return false, // 如果路径不是有效 UTF-8，通常认为不匹配
        };

        // 2. 使用 `at` 方法查找路由
        //    注意：`matchit::Router::at` 返回 `Result<Match, NoMatch>`
        match self.paths.at(path_str) {
            Ok(matched) => *matched.value, // 如果匹配成功，返回关联的 bool 值
            Err(_) => false,               // 匹配失败（路径不存在或未注册）
        }
    }
}