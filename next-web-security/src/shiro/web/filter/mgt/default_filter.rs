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
    path_config_processor::PathConfigProcessor,
    path_matching_filter::{PathMatchingFilter, PathMatchingFilterExt},
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

    pub fn new_instance_and_process_path_config(
        name: impl AsRef<str>,
        path: &str,
        config: Option<&str>,
    ) -> Option<Box<dyn HttpFilter>> {
        if let Some(filter) = Self::from_str(name.as_ref()) {
            let filter = match filter {
                DefaultFilter::Anon => Self::defalt_instance::<AnonymousFilter>(path, config),
                DefaultFilter::Authc => {
                    Self::defalt_instance::<FormAuthenticationFilter>(path, config)
                }
                DefaultFilter::AuthcBasic => {
                    Self::defalt_instance::<BasicHttpAuthenticationFilter>(path, config)
                }
                DefaultFilter::AuthcBearer => {
                    Self::defalt_instance::<BearerHttpAuthenticationFilter>(path, config)
                }
                DefaultFilter::Ip => Self::defalt_instance::<IpFilter>(path, config),
                DefaultFilter::Logout => Self::wapper::<LogoutFilter>(),
                DefaultFilter::NoSessionCreation => {
                    Self::defalt_instance::<NoSessionCreationFilter>(path, config)
                }
                DefaultFilter::Perms => {
                    Self::defalt_instance::<PermissionsAuthorizationFilter>(path, config)
                }
                DefaultFilter::Port => Self::defalt_instance::<PortFilter>(path, config),
                DefaultFilter::Rest => {
                    Self::defalt_instance::<HttpMethodPermissionFilter>(path, config)
                }
                DefaultFilter::Roles => {
                    Self::defalt_instance::<RolesAuthorizationFilter>(path, config)
                }
                DefaultFilter::SsL => Self::defalt_instance::<SslFilter>(path, config),
                DefaultFilter::User => Self::defalt_instance::<UserFilter>(path, config),
                DefaultFilter::InvalidRequest => {
                    Self::defalt_instance::<InvalidRequestFilter>(path, config)
                }
            };

            return Some(filter);
        }

        None
    }

    fn defalt_instance<T>(path: &str, config: Option<&str>) -> Box<dyn HttpFilter>
    where
        T: Clone + Default + 'static,
        T: Required<PathMatchingFilter> + Required<OncePerRequestFilter>,
        T: AdviceFilterExt + PathMatchingFilterExt,
        T: Named,
    {
        let mut filter = T::default();
        if let Some(config) = config {
            filter.process_path_config(path, config);
        }

        Box::new(HttpFilterWrapper(filter))
    }

    fn wapper<T>() -> Box<dyn HttpFilter>
    where
        T: Clone + Default + 'static,
        T: Required<OncePerRequestFilter>,
        T: AdviceFilterExt + PathMatchingFilterExt,
        T: Named,
    {
        Box::new(HttpFilterWrapper(T::default()))
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "anon" => Self::Anon.into(),
            "authc" => Self::Authc.into(),
            "authcBasic" => Self::AuthcBasic.into(),
            "authcBearer" => Self::AuthcBearer.into(),
            "ip" => Self::Ip.into(),
            "logout" => Self::Logout.into(),
            "noSessionCreation" => Self::NoSessionCreation.into(),
            "perms" => Self::Perms.into(),
            "port" => Self::Port.into(),
            "rest" => Self::Rest.into(),
            "roles" => Self::Roles.into(),
            "ssl" => Self::SsL.into(),
            "user" => Self::User.into(),
            "invalidRequest" => Self::InvalidRequest.into(),
            _ => None,
        }
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
