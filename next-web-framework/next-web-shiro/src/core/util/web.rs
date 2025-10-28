use next_web_core::traits::http::http_request::HttpRequest;
use next_web_core::traits::http::http_response::HttpResponse;

use crate::core::subject::{self, Subject};

pub struct WebUtils;

impl WebUtils {
    pub fn redirect_to_saved_request(req: &mut dyn HttpRequest, res: &mut dyn HttpResponse) {}

    pub fn save_request(req: &dyn HttpRequest, subject: &dyn Subject) {}

    pub fn issue_redirect(req: &mut dyn HttpRequest, res: &mut dyn HttpResponse, login_url: &str) {
        
    }
}
