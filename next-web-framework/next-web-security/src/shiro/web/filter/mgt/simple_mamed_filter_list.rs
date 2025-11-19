use next_web_core::{clone_box, traits::{filter::{http_filter::HttpFilter, http_filter_chain::HttpFilterChain}, required::Required}};

use crate::web::{filter::mgt::named_filter_list::NamedFilterList, proxied_filter_chain::ProxiedFilterChain};


#[derive(Clone)]
pub struct SimpleNamedFilterList {
    name: String,
    backing_list: Vec<Box<dyn HttpFilter>>
}

impl SimpleNamedFilterList {
    
    pub fn new<T: ToString>(name: T) -> Self {
        Self {
            name: name.to_string(),
            backing_list: Vec::new(),
        }
    }

    pub fn set_name<T: ToString>(&mut self, name: T) {
        let name = name.to_string();
        assert!(!name.is_empty(), "Cannot specify a null or empty name.");
        
        self.name = name;
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }


}

impl NamedFilterList for SimpleNamedFilterList {
    fn name(&self) -> &str {
        &self.name
    }

    fn proxy(&self, orig: & dyn HttpFilterChain) -> Box<dyn HttpFilterChain> {
        Box::new(ProxiedFilterChain::new(clone_box(orig), self.get_object().clone()))
    }
}

impl Required<Vec<Box<dyn HttpFilter>>> for SimpleNamedFilterList {
    fn get_object(&self) -> & Vec<Box<dyn HttpFilter>> {
        &self.backing_list
    }

    fn get_mut_object(&mut self) -> &mut Vec<Box<dyn HttpFilter>> {
        &mut self.backing_list
    }
}