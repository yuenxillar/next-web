use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};

use crate::{core::util::object::Object, web::filter::{
    access_control_filter::AccessControlFilterExt, authz::authorization_filter::AuthorizationFilter,
}};

#[derive(Clone)]
pub struct RolesAuthorizationFilter {
    authorization_filter: AuthorizationFilter,
}

impl AccessControlFilterExt for RolesAuthorizationFilter {
    fn is_access_allowed(
        &self,
        request: &dyn HttpRequest,
        response: &dyn HttpResponse,
        mapped_value: &Object,
    ) -> bool {
        todo!()
    }
}

impl Default for RolesAuthorizationFilter {
    fn default() -> Self {
        Self { authorization_filter: todo!() }
    }
}
