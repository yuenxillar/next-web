use std::collections::HashMap;

use next_web_core::{
    http::cookie::Cookie,
    util::{http_method::HttpMethod, locale::Locale},
};

use crate::web::savedrequest::SavedRequest;

#[derive(Debug, Clone)]
pub struct DefaultSavedRequest {}

impl SavedRequest for DefaultSavedRequest {
    fn get_redirect_url(&self) -> String {
        todo!()
    }

    fn get_cookies(&self) -> Vec<Cookie> {
        todo!()
    }

    fn get_method(&self) -> String {
        todo!()
    }

    fn get_header_values(&self, name: &str) -> Vec<String> {
        todo!()
    }

    fn get_header_names(&self) -> Vec<String> {
        todo!()
    }

    fn get_locales(&self) -> Vec<Locale> {
        todo!()
    }

    fn get_parameter_values(&self, name: &str) -> Vec<String> {
        todo!()
    }

    fn get_parameter_map(&self) -> HashMap<String, Vec<String>> {
        todo!()
    }
}

impl DefaultSavedRequest {
    pub fn builder() -> DefaultSavedRequestBuilder {
        DefaultSavedRequestBuilder::default()
    }
}

pub struct DefaultSavedRequestBuilder {}

impl Default for DefaultSavedRequestBuilder {
    fn default() -> Self {
        Self {}
    }
}

impl DefaultSavedRequestBuilder {
    pub fn build(&mut self) -> DefaultSavedRequest {
        DefaultSavedRequest {}
    }

    pub fn set_scheme(&mut self, scheme: Option<String>) -> &mut Self {
        self
    }

    pub fn set_server_name(&mut self, server_name: Option<String>) -> &mut Self {
        self
    }

    pub fn set_request_uri(&mut self, request_uri: String) -> &mut Self {
        self
    }

    pub fn set_query_string(&mut self, query_string: Option<String>) -> &mut Self {
        self
    }

    pub fn set_server_port(&mut self, port: u16) -> &mut Self {
        self
    }

    pub fn set_method(&mut self, method: HttpMethod) -> &mut Self {
        self
    }

    pub fn set_locales(&mut self, locales: Option<Vec<&Locale>>) -> &mut Self {
        self
    }

    pub fn set_parameters(&mut self, parameters: Option<Vec<(&str, &str)>>) -> &mut Self {
        self
    }
}
