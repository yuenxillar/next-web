use next_web_core::traits::{filter::http_filter::HttpFilter, named::Named, required::Required};

use crate::web::filter::{
    advice_filter::AdviceFilterExt,
    authc::{
        anonymous_filter::AnonymousFilter,
        basic_http_authentication_filter::BasicHttpAuthenticationFilter,
        bearer_http_authentication_filter::BearerHttpAuthenticationFilter,
        form_authentication_filter::FormAuthenticationFilter, logout_filter::LogoutFilter,
        user_filter::UserFilter,
    },
    authz::{
        http_method_permission_filter::HttpMethodPermissionFilter, ip_filter::IpFilter,
        permissions_authorization_filter::PermissionsAuthorizationFilter, port_filter::PortFilter,
        roles_authorization_filter::RolesAuthorizationFilter, ssl_filter::SslFilter,
    },
    invalid_request_filter::InvalidRequestFilter,
    once_per_request_filter::{HttpFilterWrapper, OncePerRequestFilter},
    session::no_session_creation_filter::NoSessionCreationFilter,
};

#[derive(Clone, Default)]
pub enum DefaultFilter {
    Anon,
    Authc,
    AuthcBasic,
    AuthcBearer,
    Ip,
    Logout,
    NoSessionCreation,
    Perms,
    Port,
    Rest,
    Roles,
    SsL,
    User,
    #[default]
    InvalidRequest,
}

impl DefaultFilter {
    pub fn new_instance(self) -> Box<dyn HttpFilter> {
        BasicHttpAuthenticationFilter::default();
        match self {
            DefaultFilter::Anon => Self::wapper::<AnonymousFilter>(),
            DefaultFilter::Authc => Self::wapper::<FormAuthenticationFilter>(),
            DefaultFilter::AuthcBasic => Self::wapper::<BasicHttpAuthenticationFilter>(),
            DefaultFilter::AuthcBearer => Self::wapper::<BearerHttpAuthenticationFilter>(),
            DefaultFilter::Ip => Self::wapper::<IpFilter>(),
            DefaultFilter::Logout => Self::wapper::<LogoutFilter>(),
            DefaultFilter::NoSessionCreation => Self::wapper::<NoSessionCreationFilter>(),
            DefaultFilter::Perms => Self::wapper::<PermissionsAuthorizationFilter>(),
            DefaultFilter::Port => Self::wapper::<PortFilter>(),
            DefaultFilter::Rest => Self::wapper::<HttpMethodPermissionFilter>(),
            DefaultFilter::Roles => Self::wapper::<RolesAuthorizationFilter>(),
            DefaultFilter::SsL => Self::wapper::<SslFilter>(),
            DefaultFilter::User => Self::wapper::<UserFilter>(),
            DefaultFilter::InvalidRequest => Self::wapper::<InvalidRequestFilter>(),
        }
    }

    fn wapper<T>() -> Box<dyn HttpFilter>
    where
        T: Clone + Default + 'static,
        T: Named + Required<OncePerRequestFilter>,
        T: AdviceFilterExt,
    {
        Box::new(HttpFilterWrapper(T::default()))
    }

    pub fn name(&self) -> &'static str {
        match self {
            DefaultFilter::Anon => "anon",
            DefaultFilter::Authc => "authc",
            DefaultFilter::AuthcBasic => "authcBasic",
            DefaultFilter::AuthcBearer => "authcBearer",
            DefaultFilter::Ip => "ip",
            DefaultFilter::Logout => "logout",
            DefaultFilter::NoSessionCreation => "noSessionCreation",
            DefaultFilter::Perms => "perms",
            DefaultFilter::Port => "port",
            DefaultFilter::Rest => "rest",
            DefaultFilter::Roles => "roles",
            DefaultFilter::SsL => "ssl",
            DefaultFilter::User => "user",
            DefaultFilter::InvalidRequest => "invalidRequest",
        }
    }
    pub fn values() -> Vec<DefaultFilter> {
        vec![
            Self::Anon,
            Self::Authc,
            Self::AuthcBasic,
            Self::AuthcBearer,
            Self::Ip,
            Self::Logout,
            Self::NoSessionCreation,
            Self::Perms,
            Self::Port,
            Self::Rest,
            Self::Roles,
            Self::SsL,
            Self::User,
            Self::InvalidRequest,
        ]
    }
}
