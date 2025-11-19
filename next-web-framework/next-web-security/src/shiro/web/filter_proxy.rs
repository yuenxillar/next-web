use std::{any::Any, collections::HashMap, sync::Arc};

use crate::{
    core::{mgt::security_manager::SecurityManager, subject::Subject},
    web::{
        filter::{
            access_control_filter::AccessControlFilter,
            authc::authentication_filter::AuthenticationFilter,
            authz::authorization_filter::AuthorizationFilter,
            mgt::{
                default_filter_chain_manager::DefaultFilterChainManager,
                filter_chain_manager::FilterChainManager,
                path_matching_filter_chain_resolver::PathMatchingFilterChainResolver,
            },
        },
        mgt::default_web_security_manager::DefaultWebSecurityManager,
        subject::support::web_delegating_subject::WebDelegatingSubject,
    },
};

use indexmap::IndexMap;
use next_web_core::{
    async_trait,
    error::BoxError,
    traits::{
        filter::{http_filter::HttpFilter, http_filter_chain::HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
};
use tracing::error;

#[derive(Clone)]
pub struct FilterProxy {
    security_manager: Arc<dyn SecurityManager>,
    filter_chain_resolver: Option<PathMatchingFilterChainResolver>,
    global_filters: Vec<String>,

    filters: HashMap<String, Box<dyn HttpFilter>>,
    login_url: Option<String>,
    success_url: Option<String>,
    unauthorized_url: Option<String>,
}

impl FilterProxy {
    pub fn new<S>(security_manager: S) -> Self
    where
        S: SecurityManager + 'static,
    {
        let mut proxy = Self {
            filter_chain_resolver: None,
            security_manager: Arc::new(security_manager),
            filters: Default::default(),
            login_url: Default::default(),
            success_url: Default::default(),
            unauthorized_url: Default::default(),
            global_filters: Default::default(),
        };
        let manager = proxy.create_filter_chain_manager();
        let mut filter_chain_resolver = PathMatchingFilterChainResolver::default();
        filter_chain_resolver.set_filter_chain_manager(manager);

        proxy.filter_chain_resolver = Some(filter_chain_resolver);

        proxy
    }

    pub fn add_filter<K, V>(&mut self, name: K, filter: V)
    where
        K: ToString,
        V: HttpFilter + 'static,
    {
        self.filters.insert(name.to_string(), Box::new(filter));
    }

    pub fn set_login_url<T>(&mut self, login_url: T)
    where
        T: ToString,
    {
        self.login_url = Some(login_url.to_string());
    }

    pub fn set_success_url<T>(&mut self, success_url: T)
    where
        T: ToString,
    {
        self.success_url = Some(success_url.to_string());
    }

    pub fn set_unauthorized_url<T>(&mut self, unauthorized_url: T)
    where
        T: ToString,
    {
        self.unauthorized_url = Some(unauthorized_url.to_string());
    }

    pub fn get_login_url(&self) -> Option<&str> {
        self.login_url.as_deref()
    }

    pub fn get_success_url(&self) -> Option<&str> {
        self.success_url.as_deref()
    }

    pub fn get_unauthorized_url(&self) -> Option<&str> {
        self.unauthorized_url.as_deref()
    }

    pub fn get_security_manager(&self) -> &Arc<dyn SecurityManager> {
        &self.security_manager
    }

    pub fn get_mut_filters(&mut self) -> &mut HashMap<String, Box<dyn HttpFilter>> {
        &mut self.filters
    }

    pub fn get_filters(&self) -> &HashMap<String, Box<dyn HttpFilter>> {
        &self.filters
    }

    pub fn set_filters(&mut self, filters: HashMap<String, Box<dyn HttpFilter>>) {
        self.filters = filters;
    }

    pub fn create_filter_chain_manager(&mut self) -> DefaultFilterChainManager {
        let mut manager = DefaultFilterChainManager::default();
        let default_filters = manager.get_filters();
        for (_, filter) in default_filters {
            self.apply_global_properties_if_necessary(filter.as_mut());
        }

        let filters = self.get_filters().clone();
        if !filters.is_empty() {
            for (name, mut filter) in filters {
                self.apply_global_properties_if_necessary(filter.as_mut());

                manager.add_filter(name, filter, false);
            }
        }

        // set the global filters
        manager.set_global_filters(self.global_filters.clone());

        // build up the chains:
        let chains: IndexMap<String, String> = IndexMap::new();
        if !chains.is_empty() {
            for (url, chain_definition) in chains {
                manager.create_chain(url, chain_definition);
            }
        }

        // create the default chain, to match anything the path matching would have missed
        // TODO this assumes ANT path matching, which might be OK here
        manager.create_default_chain("/**".into());

        manager
    }

    fn apply_global_properties_if_necessary(&self, filter: &mut dyn HttpFilter) {
        self.apply_login_url_if_necessary(filter);
        self.apply_success_url_if_necessary(filter);
        self.apply_unauthorized_url_if_necessary(filter);
    }

    fn apply_login_url_if_necessary(&self, filter: &mut dyn HttpFilter) {
        let login_url = self.get_login_url();
        if let Some(url) = login_url {
            if !url.is_empty() && filter.supports("AccessControlFilter") {
                if let Some(ac_filter) =
                    (filter as &mut dyn Any).downcast_mut::<AccessControlFilter>()
                {
                    let existing_login_url = ac_filter.get_login_url();
                    if AccessControlFilter::<()>::DEFAULT_LOGIN_URL.eq(existing_login_url) {
                        ac_filter.set_login_url(url);
                    }
                }
            }
        }
    }

    fn apply_success_url_if_necessary(&self, filter: &mut dyn HttpFilter) {
        let success_url = self.get_success_url();
        if let Some(url) = success_url {
            if !url.is_empty() && filter.supports("AccessControlFilter") {
                if let Some(authc_filter) =
                    (filter as &mut dyn Any).downcast_mut::<AuthenticationFilter>()
                {
                    let existing_success_url = authc_filter.get_success_url();
                    if AuthenticationFilter::DEFAULT_SUCCESS_URL.eq(existing_success_url) {
                        authc_filter.set_success_url(url);
                    }
                }
            }
        }
    }

    fn apply_unauthorized_url_if_necessary(&self, filter: &mut dyn HttpFilter) {
        let unauthorized_url = self.get_unauthorized_url();
        if let Some(url) = unauthorized_url {
            if !url.is_empty() && filter.supports("AccessControlFilter") {
                if let Some(authz_filter) =
                    (filter as &mut dyn Any).downcast_mut::<AuthorizationFilter>()
                {
                    let existing_unauthorized_url = authz_filter.get_unauthorized_url();
                    if let None = existing_unauthorized_url {
                        authz_filter.set_unauthorized_url(url);
                    }
                }
            }
        }
    }

    pub fn create_subject(&self) -> WebDelegatingSubject {
        // WebDelegatingSubject::from(value)
        todo!()
    }

    pub fn get_filter_chain_resolver(&self) -> Option<&PathMatchingFilterChainResolver> {
        self.filter_chain_resolver.as_ref()
    }
    pub fn get_execution_chain(
        &self,
        req: &dyn HttpRequest,
        res: &dyn HttpResponse,
        orig_chain: &dyn HttpFilterChain,
    ) -> Option<Box<dyn HttpFilterChain>> {
        let resolver = self.get_filter_chain_resolver();
        if resolver.is_none() {
            return None;
        }

        let resolver = match resolver {
            Some(resolver) => resolver,
            None => return None,
        };

        resolver.get_chain(req, res, orig_chain)
    }

    #[allow(unused_variables)]
    pub fn update_session_last_access_time(
        &self,
        req: &dyn HttpRequest,
        res: &dyn HttpResponse,
        subject: &mut dyn Subject,
    ) {
        let session = subject.get_session();
        if let Some(session) = session {
            if let Err(err) = session.touch() {
                error!(
                    "session.touch() method invocation has failed.  Unable to update the corresponding session's last access time based on the incoming request. error: {:?}",
                    err
                )
            }
        }
    }
    pub async fn execute_chain<'a>(
        &'a self,
        req: &mut dyn HttpRequest,
        res: &mut dyn HttpResponse,
        orig_chain: &'a dyn HttpFilterChain,
    ) -> Result<(), BoxError> {
        match self.get_execution_chain(req, res, orig_chain) {
            Some(chain) => chain.do_filter(req, res).await,
            None => orig_chain.do_filter(req, res).await,
        }
    }
}

impl Named for FilterProxy {
    fn name(&self) -> &str {
        "FilterProxy"
    }
}

#[async_trait]
impl HttpFilter for FilterProxy {
    async fn do_filter(
        &self,
        req: &mut dyn HttpRequest,
        res: &mut dyn HttpResponse,
        orig_chain: &dyn HttpFilterChain,
    ) -> Result<(), BoxError> {
        req.ready();

        let mut subject = self.create_subject();

        self.update_session_last_access_time(req, res, &mut subject);
        self.execute_chain(req, res, orig_chain).await?;

        req.clean_up();
        Ok(())
    }
}

impl Default for FilterProxy {
    fn default() -> Self {
        Self::new(DefaultWebSecurityManager::default())
    }
}
