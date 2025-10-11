use std::{collections::HashMap, sync::Arc};

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use next_web_core::{
    anys::any_value::AnyValue,
    traits::{ordered::Ordered, Required::Required},
    ApplicationContext,
};

use crate::{
    authorization::authentication_manager::AuthenticationManager,
    config::{
        authentication::builders::authentication_manager_builder::AuthenticationManagerBuilder,
        object_post_processor::ObjectPostProcessor,
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
    permission::model::permission_group::PermissionGroup,
    web::default_security_filter_chain::DefaultSecurityFilterChain,
};

type BoxError = Box<dyn std::error::Error>;
type ErrorHandler = Box<dyn FnOnce(BoxError) -> Response + Send + Sync>;

pub struct HttpSecurity {
    pub(crate) any_match: Vec<(&'static str, PermissionGroup)>,
    pub(crate) not_match: Vec<&'static str>,
    pub(crate) match_type: MatchType,
    pub(crate) error_handler: ErrorHandler,

    request_matcher_configurer: RequestMatcherConfigurer,
    filters: Vec<OrderedFilter>,
    request_matcher: Arc<dyn RequestMatcher>,
    filter_orders: FilterOrderRegistration,
    authentication_manager: Arc<dyn AuthenticationManager>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum MatchType {
    #[default]
    AllMatch,
    OnlyMatchOwner,
    NotMatch,
}

impl HttpSecurity {
    pub fn new(
        object_post_processor: Arc<dyn ObjectPostProcessor>,
        authentication_builder: AuthenticationManagerBuilder,
        shared_objects: HashMap<String, AnyValue>,
    ) -> Self {
        Self {
            any_match: Vec::new(),
            match_type: MatchType::default(),
            not_match: Vec::new(),
            error_handler: Box::new(|_| (StatusCode::UNAUTHORIZED, "Unauthorized").into_response()),

            filter_orders: Default::default(),
            request_matcher: AnyRequestMatcher::default(),
            filters: Default::default(),
        }
    }

    pub fn any_match<F>(mut self, path: &'static str, f: F) -> Self
    where
        F: FnOnce(PermissionGroup) -> PermissionGroup,
    {
        let permission_group = f(PermissionGroup::default());
        self.any_match.push((path, permission_group));
        self
    }

    pub fn not_match(mut self, path: &'static str) -> Self {
        self.not_match.push(path);
        self
    }

    pub fn not_matches<P>(mut self, paths: P) -> Self
    where
        P: IntoIterator<Item = &'static str>,
    {
        for path in paths {
            self.not_match.push(path);
        }
        self
    }

    pub fn map_error<F>(mut self, f: F) -> Self
    where
        F: FnOnce(BoxError) -> Response + Send + Sync,
        F: 'static,
    {
        self.error_handler = Box::new(f);
        self
    }

    pub fn disable(mut self) -> Self {
        self.match_type = MatchType::OnlyMatchOwner;
        self
    }

    pub fn disable_all(mut self) -> Self {
        self.match_type = MatchType::NotMatch;
        self.any_match.clear();
        self.not_match.clear();
        self
    }
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

    fn get_or_apply<C>(&self, mut configurer: C) -> C
    where
        C: Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, Self>>,
    {
        let existing_config = self.get_configurer::<C>();
        match existing_config {
            Some(existing_config) => existing_config,
            None => self.with(configurer),
        }
    }

    fn get_context(&self) -> &ApplicationContext {
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
}

impl Clone for HttpSecurity {
    fn clone(&self) -> Self {
        Self {
            any_match: self.any_match.clone(),
            not_match: self.not_match.clone(),
            match_type: self.match_type.clone(),
            error_handler: Box::new(|_| (StatusCode::UNAUTHORIZED, "Unauthorized").into_response()),
        }
    }
}

impl Default for HttpSecurity {
    fn default() -> Self {
        Self {
            any_match: Default::default(),
            not_match: Default::default(),
            match_type: MatchType::NotMatch,
            error_handler: Box::new(|_| (StatusCode::UNAUTHORIZED, "Unauthorized").into_response()),
        }
    }
}

#[derive(Clone)]
pub struct RequestMatcherConfigurer {}

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
