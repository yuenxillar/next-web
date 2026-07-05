use std::sync::Arc;

use next_web_core::{
    async_trait,
    http::Cookie,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
    util::StringUtils,
};

use crate::{core::Authentication, web::authentication::logout::LogoutHandler};

type AddCookieFn = Arc<dyn Fn((Option<&str>, bool)) -> Cookie + Send + Sync>;

#[derive(Clone)]
pub struct CookieClearingLogoutHandler {
    cookies_to_clear: Vec<AddCookieFn>,
}

impl CookieClearingLogoutHandler {
    pub fn new(cookies_to_clear: Vec<String>) -> Self {
        let cookies_to_clear = cookies_to_clear
            .into_iter()
            .map(|cookie_name| {
                Arc::new(move |(context_path, is_secure): (Option<&str>, bool)| {
                    let mut cookie = Cookie::new(&cookie_name, None);

                    let path = context_path
                        .filter(|s| StringUtils::has_text(s))
                        .unwrap_or("/");
                    cookie.set_path(path);
                    cookie.set_max_age(0);
                    cookie.set_secure(is_secure);
                    cookie
                }) as AddCookieFn
            })
            .collect::<Vec<_>>();
        Self { cookies_to_clear }
    }
}

#[async_trait]
impl LogoutHandler for CookieClearingLogoutHandler {
    async fn logout(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        _authentication: Option<&Arc<dyn Authentication>>,
    ) {
        self.cookies_to_clear.iter().for_each(|f| {
            response.add_cookie(f((request.context_path(), request.is_secure())));
        });
    }
}
