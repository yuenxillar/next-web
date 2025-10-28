use crate::web::filter::{
    access_control_filter::AccessControlFilterExt, authz::authorization_filter::AuthorizationFilter,
};

#[derive(Clone)]
pub struct RolesAuthorizationFilter {
    authorization_filter: AuthorizationFilter,
}

impl AccessControlFilterExt for RolesAuthorizationFilter {
    fn is_access_allowed(
        &self,
        request: &dyn next_web_core::traits::http::http_request::HttpRequest,
        response: &dyn next_web_core::traits::http::http_response::HttpResponse,
    ) -> bool {
        todo!()
    }
}

impl Default for RolesAuthorizationFilter {
    fn default() -> Self {
        Self { authorization_filter: todo!() }
    }
}
