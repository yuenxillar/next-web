use std::{collections::HashMap, sync::Arc};

use next_web_core::traits::filter::{http_filter::HttpFilter, http_filter_chain::HttpFilterChain};

use crate::web::filter::mgt::filter_chain_manager::FilterChainManager;

pub struct DefaultFilterChainManager {
    filters: HashMap<String, Box<dyn HttpFilter>>,
}

impl DefaultFilterChainManager {
    pub fn get_filters(&mut self) -> &mut HashMap<String, Box<dyn HttpFilter>> {
        &mut self.filters
    }

    pub fn add_filter(
        &mut self,
        name: impl ToString,
        filter: Box<dyn HttpFilter>,
        overwrite: bool,
    ) {
        let name = name.to_string();
        if !self.filters.contains_key(&name) || overwrite {
            self.filters.insert(name, filter);
        }
    }

    pub fn init_filter(&mut self, filter: impl ToString) {}

}

impl FilterChainManager for DefaultFilterChainManager {
    fn get_filters(&self) -> &mut HashMap<String, Box<dyn HttpFilter>> {
        todo!()
    }

    fn get_chain(&self, chain_name: &str) {
        todo!()
    }

    fn has_chains(&self) -> bool {
        todo!()
    }

    fn get_chain_names(&self) -> Vec<&str> {
        todo!()
    }

    fn add_filter(&mut self, name: String, filter: Arc<dyn HttpFilter>) {
        todo!()
    }

    fn create_chain(&mut self, chain_name: String, chain_definition: String) {
        todo!()
    }

    fn create_default_chain(&mut self, chain_name: String) {
        todo!()
    }

    fn add_to_chain(&mut self, chain_name: &str, filter_name: String) {
        todo!()
    }

    fn set_global_filters(&mut self, global_filter_names: Vec<String>) {
        todo!()
    }

    fn proxy(
        &self,
        original: &dyn HttpFilterChain,
        chain_name: &str,
    ) -> Option<&dyn HttpFilterChain> {
        todo!()
    }
}

impl Default for DefaultFilterChainManager {
    fn default() -> Self {
        Self {
            filters: HashMap::new(),
        }
    }
}
