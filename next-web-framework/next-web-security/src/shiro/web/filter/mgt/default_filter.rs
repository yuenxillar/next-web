use std::sync::Arc;

use next_web_core::traits::filter::http_filter::HttpFilter;

use crate::web::filter::authz::roles_authorization_filter::RolesAuthorizationFilter;


#[derive(Clone, Default)]
pub enum DefaultFilter {
    Anon,
    Authc,
    AuthcBasic,
    AuthcBearer,
    Ip,
    Logout,
    Port,
    Roles(RolesAuthorizationFilter),
    SSL,
    User,
    #[default]
    InvalidRequest,
}

impl DefaultFilter {

    pub fn new_instance(self) -> Arc<dyn HttpFilter> {
        match self {
            DefaultFilter::Anon => todo!(),
            DefaultFilter::Authc => todo!(),
            DefaultFilter::AuthcBasic => todo!(),
            DefaultFilter::AuthcBearer => todo!(),
            DefaultFilter::Ip => todo!(),
            DefaultFilter::Logout => todo!(),
            DefaultFilter::Port => todo!(),
            DefaultFilter::Roles(roles_authorization_filter) => todo!(),
            DefaultFilter::SSL => todo!(),
            DefaultFilter::User => todo!(),
            DefaultFilter::InvalidRequest => todo!(),
        }
    }
}