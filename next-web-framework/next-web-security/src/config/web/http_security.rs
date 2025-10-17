use std::{borrow::Cow, sync::Arc};

use axum::response::Response;
use next_web_core::{
    anys::any_map::AnyMap,
    traits::{any_clone::AnyClone, ordered::Ordered, required::Required},
    ApplicationContext,
};

use crate::{
    authorization::authentication_manager::AuthenticationManager,
    config::{
        abstract_configured_security_builder::AbstractConfiguredSecurityBuilder,
        authentication::builders::authentication_manager_builder::AuthenticationManagerBuilder,
        security_builder::SecurityBuilder,
        security_configurer::SecurityConfigurer,
        security_configurer_adapter::SecurityConfigurerAdapter,
        web::{
            builders::filter_order_registration::FilterOrderRegistration,
            configurers::{
                abstract_authentication_filter_configurer::AuthenticationFilterConfigurer,
                authorize_http_requests_configurer::{
                    AuthorizationManagerRequestMatcherRegistry, AuthorizeHttpRequestsConfigurer,
                },
                form_login_configurer::FormLoginConfigurer,
            },
            http_security_builder::HttpSecurityBuilder,
            util::matcher::{
                any_request_matcher::AnyRequestMatcher, request_matcher::RequestMatcher,
            },
        },
    },
    core::filter::Filter,
    web::default_security_filter_chain::DefaultSecurityFilterChain,
};

type BoxError = Box<dyn std::error::Error>;
type ErrorHandler = Box<dyn FnOnce(BoxError) -> Response + Send + Sync>;

pub struct HttpSecurity {
    // pub(crate) any_match: Vec<(&'static str, PermissionGroup)>,
    // pub(crate) not_match: Vec<&'static str>,
    // pub(crate) match_type: MatchType,
    // pub(crate) error_handler: ErrorHandler,

    request_matcher_configurer: RequestMatcherConfigurer,
    filters: Vec<OrderedFilter>,
    request_matcher: Arc<dyn RequestMatcher>,
    filter_orders: FilterOrderRegistration,
    authentication_manager: Option<Arc<dyn AuthenticationManager>>,

    abstract_configured_security_builder:
        AbstractConfiguredSecurityBuilder<DefaultSecurityFilterChain, Self>,
}

// #[derive(Debug, Clone, PartialEq, Eq, Default)]
// pub enum MatchType {
//     #[default]
//     AllMatch,
//     OnlyMatchOwner,
//     NotMatch,
// }

impl HttpSecurity {
    pub fn new(
        authentication_builder: AuthenticationManagerBuilder,
        shared_objects: AnyMap,
    ) -> Self {
        let abstract_configured_security_builder = AbstractConfiguredSecurityBuilder::new(false);
        let http_security = Self {
            filter_orders: Default::default(),
            request_matcher: Arc::new(AnyRequestMatcher::default()),
            filters: Default::default(),
            request_matcher_configurer: RequestMatcherConfigurer::new(),
            authentication_manager: Default::default(),
            abstract_configured_security_builder,
        };

        http_security.set_shared_object(
            std::any::type_name::<AuthenticationManagerBuilder>(),
            authentication_builder,
        );

        shared_objects.for_each(|name, object| {
            http_security.set_shared_object(name.to_string(), object.to_owned());
        });

        http_security
    }

    fn get_context(&self) -> &ApplicationContext {
        todo!()
    }

    // pub fn any_match<F>(mut self, path: &'static str, f: F) -> Self
    // where
    //     F: FnOnce(PermissionGroup) -> PermissionGroup,
    // {
    //     let permission_group = f(PermissionGroup::default());
    //     self.any_match.push((path, permission_group));
    //     self
    // }

    // pub fn not_match(mut self, path: &'static str) -> Self {
    //     self.not_match.push(path);
    //     self
    // }

    // pub fn not_matches<P>(mut self, paths: P) -> Self
    // where
    //     P: IntoIterator<Item = &'static str>,
    // {
    //     for path in paths {
    //         self.not_match.push(path);
    //     }
    //     self
    // }

    // pub fn map_error<F>(mut self, f: F) -> Self
    // where
    //     F: FnOnce(BoxError) -> Response + Send + Sync,
    //     F: 'static,
    // {
    //     self.error_handler = Box::new(f);
    //     self
    // }

    // pub fn disable(mut self) -> Self {
    //     self.match_type = MatchType::OnlyMatchOwner;
    //     self
    // }

    // pub fn disable_all(mut self) -> Self {
    //     self.match_type = MatchType::NotMatch;
    //     self.any_match.clear();
    //     self.not_match.clear();
    //     self
    // }
}

impl HttpSecurity {
    pub fn authorize_http_requests<F>(self, authorize_http_requests_configurer: F) -> Self
    where
        F: FnOnce(AuthorizationManagerRequestMatcherRegistry),
    {
        authorize_http_requests_configurer(
            self.get_or_apply(AuthorizeHttpRequestsConfigurer::new(self.get_context()))
                .get_registry(),
        );
        self
    }

    pub fn form_login<F>(self, form_login: F) -> Self
    where
        F: FnOnce(FormLoginConfigurer<HttpSecurity>),
    {
        form_login(self.get_or_apply(FormLoginConfigurer::default()));
        self
    }

    // pub fn headers(self) -> Self {
    // }

    // pub fn cors(self) -> Self {
    // }

    // pub fn session_management(self) -> Self {
    // }

    // pub fn port_mapper(self) -> Self {
    // }

    // pub fn x509(self) -> Self {
    // }

    fn get_or_apply<C>(&self, mut configurer: C) -> C
    where
        C: Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, Self>>,
    {
        // let existing_config = self.get_configurer::<C>();
        // match existing_config {
        //     Some(existing_config) => existing_config,
        //     None => self.with(configurer),
        // }
        todo!()
    }
}

impl SecurityBuilder<DefaultSecurityFilterChain> for HttpSecurity {
    fn build(&self) -> DefaultSecurityFilterChain {
        todo!()
    }
}

impl SecurityBuilder<Self> for HttpSecurity {
    fn build(&self) -> Self {
        todo!()
    }
}

impl AuthenticationFilterConfigurer<Self> for HttpSecurity {
    fn login_processing_url(&mut self, login_processing_url: &str) {
        todo!()
    }

    fn login_page(&mut self, login_page: &str) {
        todo!()
    }
}

impl HttpSecurityBuilder<Self> for HttpSecurity {
    fn add_filter<F: Filter>(self, filter: F) -> Self {
        self
    }

    fn get_configurer<T>(&self) -> Option<T>
    where
        T: SecurityConfigurer<DefaultSecurityFilterChain, Self>,
    {
        todo!()
    }

    fn get_shared_object<T>(&self) -> Option<&T> {
        todo!()
    }

    fn get_mut_shared_object<T>(&mut self) -> Option<&mut T> {
        todo!()
    }

    fn add_filter_after<F, F1>(self, filter: F, after_filter: F1) -> Self
    where
        F: Filter,
        F1: Filter,
    {
        todo!()
    }

    fn add_filter_before<F, F1>(self, filter: F, before_filter: F1) -> Self
    where
        F: Filter,
        F1: Filter,
    {
        todo!()
    }

    fn set_shared_object<N, C>(&self, name: N, object: C)
    where
        N: Into<Cow<'static, str>>,
        C: AnyClone,
    {
        todo!()
    }
}

impl Clone for HttpSecurity {
    fn clone(&self) -> Self {
        Self {
            request_matcher_configurer: self.request_matcher_configurer.clone(),
            filters: self.filters.clone(),
            request_matcher: self.request_matcher.clone(),
            filter_orders: self.filter_orders.clone(),
            authentication_manager: self.authentication_manager.clone(),
            abstract_configured_security_builder: self.abstract_configured_security_builder.clone(),
        }
    }
}

impl Default for HttpSecurity {
    fn default() -> Self {
        Self::new(AuthenticationManagerBuilder::new(), AnyMap::new())
    }
}

#[derive(Clone)]
pub struct RequestMatcherConfigurer {
    matchers: Vec<Arc<dyn RequestMatcher>>,
}

impl RequestMatcherConfigurer {
    pub fn new() -> Self {
        Self {
            matchers: Vec::new(),
        }
    }
}

#[derive(Clone)]
pub struct OrderedFilter {
    filter: Arc<dyn Filter>,
    order: i32,
}

impl OrderedFilter {}

impl Ordered for OrderedFilter {
    fn order(&self) -> i32 {
        self.order
    }
}

impl Filter for OrderedFilter {
    fn do_filter(
        &self,
        req: &axum::extract::Request,
        res: &Response,
        next: axum::middleware::Next,
    ) {
        todo!()
    }
}
