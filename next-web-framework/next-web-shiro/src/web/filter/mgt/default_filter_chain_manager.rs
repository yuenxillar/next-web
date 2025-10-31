use std::{collections::HashMap, sync::Arc};

use next_web_core::traits::filter::http_filter::HttpFilter;


pub struct DefaultFilterChainManager {
    filters: HashMap<String, Box<dyn HttpFilter>>,
}

impl DefaultFilterChainManager {
    
    pub fn get_filters(&mut self) -> &mut  HashMap<String, Box<dyn HttpFilter>> {
        &mut self.filters
    }

    pub fn add_filter(&mut self, name: impl ToString, filter: Box<dyn HttpFilter>,  overwrite: bool) {
        let name = name.to_string();
        if !self.filters.contains_key(&name) || overwrite {
            self.filters.insert(name, filter);
        }
    }

    pub fn init_filter(&mut self, filter: impl ToString) {

    }

    pub fn create_chain(&mut self, chain_name: &str, ) {

    }
}
impl Default for DefaultFilterChainManager {
    fn default() -> Self {
        Self {
            filters: HashMap::new(),
        }
    }
}