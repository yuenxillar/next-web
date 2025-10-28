use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};

use crate::{core::util::{object::Object, web::WebUtils}, web::filter::access_control_filter::{AccessControlFilter, AccessControlFilterExt}};

#[derive(Clone)]
pub struct AuthenticationFilter {
    success_url: String,
    pub access_control_filter: AccessControlFilter,
}

impl AuthenticationFilter {
    
    pub fn get_success_url(&self) -> &str {
        &self.success_url
    }

    pub fn set_success_url(&mut self, success_url: impl ToString) {
        self.success_url = success_url.to_string();
    }

    pub fn issue_success_redirect(&self, req: &mut dyn HttpRequest, res: &mut dyn HttpResponse) {
        WebUtils::redirect_to_saved_request(req, res);
    }
}

impl AccessControlFilterExt for AuthenticationFilter {

    fn is_access_allowed(&self, request: &dyn HttpRequest, response: &dyn HttpResponse, mapped_value: &Object ) -> bool { 
        let subject = self.access_control_filter.get_subject(request, response);

        subject.is_authenticated() && subject.get_principal().is_some()
     }
}