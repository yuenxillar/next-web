use std::sync::Arc;

use crate::core::mgt::security_manager::SecurityManager;

use next_web_core::traits::filter::http_filter::HttpFilter;

#[derive(Clone)]
pub struct WebSecurityService {
    security_manager: Arc<dyn SecurityManager>,
    filters: Vec<Arc<dyn HttpFilter>>,

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

    pub fn add_filter<F>(&mut self, filter: F)
    where
        F: HttpFilter + 'static,
    {
        self.filters.push(Arc::new(filter));
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

    pub fn get_filters(&self) -> &Vec<Arc<dyn HttpFilter>> {
        &self.filters
    }

    pub fn set_filters(&mut self, filters: Vec<Arc<dyn HttpFilter>>) {
        self.filters = filters;
    }


    pub fn set_filter<F>(&mut self, filter: F) 
    where F: HttpFilter + 'static {
        self.filters.push(Arc::new(filter));
    }

    

}
