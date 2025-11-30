use std::{any::Any, collections::HashMap};

use indexmap::IndexMap;
use next_web_core::traits::filter::http_filter::HttpFilter;

use crate::web::filter::{
    access_control_filter::AccessControlFilter,
    authc::authentication_filter::AuthenticationFilter,
    authz::authorization_filter::AuthorizationFilter,
    mgt::{
        default_filter::DefaultFilter, default_filter_chain_manager::DefaultFilterChainManager,
        filter_chain_manager::FilterChainManager,
    },
};

pub struct FilterProxyConfigure {
    filters: HashMap<String, Box<dyn HttpFilter>>,
    global_filters: Vec<String>,
    filter_chain_definition_map: Option<IndexMap<String, String>>,

    login_url: Option<String>,
    success_url: Option<String>,
    unauthorized_url: Option<String>,
}

impl FilterProxyConfigure {
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

    pub fn set_filter_chain_definition_map(
        &mut self,
        filter_chain_definition_map: IndexMap<String, String>,
    ) {
        self.filter_chain_definition_map = Some(filter_chain_definition_map);
    }

    pub fn get_filter_chain_definition_map(&self) -> Option<&IndexMap<String, String>> {
        self.filter_chain_definition_map.as_ref()
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

    pub fn create_filter_chain_manager(mut self) -> DefaultFilterChainManager {
        let mut manager = DefaultFilterChainManager::default();

        for (_, filter) in manager.get_filters() {
            self.apply_global_properties_if_necessary(filter.as_mut());
        }

        let len = self.get_filters().len();
        if len > 0 {
            let filters = self.filters.drain().collect::<Vec<_>>();
            for (name, mut filter) in filters {
                self.apply_global_properties_if_necessary(filter.as_mut());

                manager.add_filter(name, filter, false);
            }
        }

        // build up the chains:
        if let Some(chains) = self.filter_chain_definition_map.take() {
            if !chains.is_empty() {
                for (url, chain_definition) in chains {
                    manager.create_chain(url, chain_definition);
                }
            }
        }

        // add necessary filters
        manager.add_necessary_filters();

        // set the global filters
        manager.set_global_filters(self.global_filters);

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
            if !url.is_empty() && filter.supports("AuthenticationFilter") {
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
            if !url.is_empty() && filter.supports("AuthorizationFilter") {
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
}

impl Default for FilterProxyConfigure {
    fn default() -> Self {
        Self {
            filters: Default::default(),
            global_filters: vec![DefaultFilter::InvalidRequest.name().to_string()],
            login_url: Some(String::from(AccessControlFilter::<()>::DEFAULT_LOGIN_URL)),
            success_url: Some(String::from("/")),
            unauthorized_url: Default::default(),
            filter_chain_definition_map: Default::default(),
        }
    }
}
