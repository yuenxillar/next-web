use std::collections::HashMap;

use next_web_core::http::filter::HttpFilter;
use next_web_core::util::indexmap::IndexMap;

use crate::web::filter::{
    access_control_filter::AccessControlFilter,
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

        let len = self.get_filters().len();
        if len > 0 {
            let filters = self.filters.drain().collect::<Vec<_>>();
            for (name, mut filter) in filters {
                self.apply_global_properties_if_necessary(filter.as_mut());

                // The filter is the property of the application, so the chains
                // use it as it is instead of creating their own instance.
                manager.add_application_filter(name, filter);
            }
        }

        // The pool of the manager holds every built-in filter before the chains
        // are created, so the global filters can be checked and run by every
        // chain.
        manager.add_necessary_filters();
        manager.set_global_filters(std::mem::take(&mut self.global_filters));

        // build up the chains:
        if let Some(chains) = self.filter_chain_definition_map.take() {
            if !chains.is_empty() {
                for (url, chain_definition) in chains {
                    manager.create_chain(url, chain_definition);
                }
            }
        }

        // create the default chain, to match anything the path matching would have missed
        // TODO this assumes ANT path matching, which might be OK here
        manager.create_default_chain("/**".into());

        // The chains hold the filters they were built for, which are not the
        // values of the manager: a filter was cloned into the pool of the
        // manager and the chain runs the other copy. The global properties of
        // the application belong to the filter of the chain, so they are
        // applied to each chain as well.
        for chain in manager.filter_chains.values_mut() {
            for filter in chain.get_mut_object().iter_mut() {
                self.apply_global_properties_if_necessary(filter.as_mut());
            }
        }

        manager
    }

    fn apply_global_properties_if_necessary(&self, filter: &mut dyn HttpFilter) {
        self.apply_login_url_if_necessary(filter);
        self.apply_success_url_if_necessary(filter);
        self.apply_unauthorized_url_if_necessary(filter);
    }

    fn apply_login_url_if_necessary(&self, filter: &mut dyn HttpFilter) {
        if let Some(url) = self.get_login_url() {
            if !url.is_empty() {
                // The filter that answers the login page takes the URL of the
                // application; every other filter has none to take.
                filter.apply_login_url(url);
            }
        }
    }

    fn apply_success_url_if_necessary(&self, filter: &mut dyn HttpFilter) {
        if let Some(url) = self.get_success_url() {
            if !url.is_empty() {
                // The filter that answers a login sends the client on to the
                // page of the application; for every other filter there is no
                // page to send it to.
                filter.apply_success_url(url);
            }
        }
    }

    fn apply_unauthorized_url_if_necessary(&self, filter: &mut dyn HttpFilter) {
        if let Some(url) = self.get_unauthorized_url() {
            if !url.is_empty() {
                // The filter that refuses a request answers with the page of
                // the application; a filter that never refuses one has none.
                filter.apply_unauthorized_url(url);
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
