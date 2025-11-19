use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use next_web_core::{
    async_trait,
    traits::{
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
        required::Required,
    },
};

use crate::{
    core::util::object::Object,
    web::filter::{
        access_control_filter::AccessControlFilterExt,
        advice_filter::AdviceFilterExt,
        authz::{
            authorization_filter::AuthorizationFilter, default_ip_source::DefaultIpSource,
            ip_address_matcher::IpAddressMatcher, ip_source::IpSource,
        },
        once_per_request_filter::OncePerRequestFilter,
    },
};

#[derive(Clone)]
pub struct IpFilter {
    ip_source: Arc<dyn IpSource>,
    authorized_ip_matchers: Vec<IpAddressMatcher>,
    denied_ip_matchers: Vec<IpAddressMatcher>,

    authorization_filter: AuthorizationFilter,
}

impl IpFilter {
    pub fn authorized_ips(&mut self, ips: &str) {
        ips.split(',')
            .map(str::trim)
            .filter(|ip| !ip.is_empty())
            .for_each(|ip| {
                self.authorized_ip_matchers
                    .push(IpAddressMatcher::new(ip).unwrap());
            });
    }

    pub fn set_denied_ips(&mut self, ips: &str) {
        ips.split(',')
            .map(str::trim)
            .filter(|ip| !ip.is_empty())
            .for_each(|ip| {
                self.denied_ip_matchers
                    .push(IpAddressMatcher::new(ip).unwrap());
            });
    }

    pub fn set_ip_source<T: IpSource + 'static>(&mut self, ip_source: T) {
        self.ip_source = Arc::new(ip_source);
    }

    pub fn get_host_from_request<'a>(&self, request: &'a dyn HttpRequest) -> Option<&'a str> {
        request.host()
    }
}

#[async_trait]
impl AccessControlFilterExt for IpFilter {
    async fn is_access_allowed(
        &self,
        request: &mut dyn HttpRequest,
        _response: &mut dyn HttpResponse,
        _mapped_value: Option<Object>,
    ) -> bool {
        let remote_ip = self.get_host_from_request(request);
        if let Some(remote_ip) = remote_ip {
            if self
                .denied_ip_matchers
                .iter()
                .any(|matcher| matcher.matches(remote_ip))
            {
                return false;
            }

            if self.ip_source.get_denied_ips().iter().any(|ip| {
                IpAddressMatcher::new(ip)
                    .map(|matcher| matcher.matches(remote_ip))
                    .unwrap_or(true)
            }) {
                return false;
            }

            if self
                .authorized_ip_matchers
                .iter()
                .any(|matcher| matcher.matches(remote_ip))
            {
                return true;
            }

            if self.ip_source.get_authorized_ips().iter().any(|ip| {
                IpAddressMatcher::new(ip)
                    .map(|matcher| matcher.matches(remote_ip))
                    .unwrap_or(false)
            }) {
                return true;
            }
        }

        false
    }
}

#[async_trait]
impl AdviceFilterExt for IpFilter {}

impl Required<OncePerRequestFilter> for IpFilter {
    fn get_object(&self) -> &OncePerRequestFilter {
        &self
            .access_control_filter
            .path_matching_filter
            .advice_filter
            .once_per_request_filter
    }

    fn get_mut_object(&mut self) -> &mut OncePerRequestFilter {
        &mut self
            .access_control_filter
            .path_matching_filter
            .advice_filter
            .once_per_request_filter
    }
}

impl Named for IpFilter {
    fn name(&self) -> &str {
        "IpFilter"
    }
}

impl Deref for IpFilter {
    type Target = AuthorizationFilter;

    fn deref(&self) -> &Self::Target {
        &self.authorization_filter
    }
}

impl DerefMut for IpFilter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.authorization_filter
    }
}

impl Default for IpFilter {
    fn default() -> Self {
        Self {
            ip_source: Arc::new(DefaultIpSource),
            authorization_filter: Default::default(),
            authorized_ip_matchers: Default::default(),
            denied_ip_matchers: Default::default(),
        }
    }
}
