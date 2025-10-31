use std::{any::Any, collections::HashMap, sync::Arc};

use crate::{core::mgt::security_manager::SecurityManager, web::filter::{access_control_filter::AccessControlFilter, authc::authentication_filter::AuthenticationFilter, authz::authorization_filter::AuthorizationFilter, mgt::default_filter_chain_manager::DefaultFilterChainManager}};

use next_web_core::{async_trait, traits::{filter::http_filter::HttpFilter, http::{http_request::HttpRequest, http_response::HttpResponse}}};

#[derive(Clone)]
pub struct WebSecurityService {
    security_manager: Arc<dyn SecurityManager>,
    filters: HashMap<String, Box<dyn HttpFilter>>,

    login_url: Option<String>,
    success_url: Option<String>,
    unauthorized_url: Option<String>,
}

impl WebSecurityService {
    pub fn new<S>(security_manager: S) -> Self
    where
        S: SecurityManager + 'static,
    {
        Self {
            security_manager: Arc::new(security_manager),
            filters: Default::default(),
            login_url: Default::default(),
            success_url: Default::default(),
            unauthorized_url: Default::default(),
        }
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

    pub fn get_security_manager(&self, username: &str, password: &str) -> & Arc<dyn SecurityManager> {
       & self.security_manager
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

    pub fn create_filter_chain_manager(&mut self) {
        let mut manager = DefaultFilterChainManager::default();
        let default_filters = manager.get_filters();
        for (_, filter) in default_filters.iter_mut() {
            self.apply_global_properties_if_necessary(filter.as_mut());
        }

        let filters = self.get_filters().clone();
        if !filters.is_empty() {
            for (name, mut filter) in filters {
                self.apply_global_properties_if_necessary(filter.as_mut());

                manager.add_filter(name, filter, false);
            }
        }

        // manager.set

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
               if let Some(ac_filter) = (filter as &mut dyn Any).downcast_mut::<AccessControlFilter>() {
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
               if let Some(authc_filter) = (filter as &mut dyn Any).downcast_mut::<AuthenticationFilter>() {
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
               if let Some(authz_filter) = (filter as &mut dyn Any).downcast_mut::<AuthorizationFilter>() {
                    let existing_unauthorized_url = authz_filter.get_unauthorized_url();
                    if let None = existing_unauthorized_url {
                        authz_filter.set_unauthorized_url(url);
                    }
               }
            }
        }
    }

}

#[async_trait]
impl HttpFilter for WebSecurityService {
 
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), String> {

        Ok(())
    }
} 