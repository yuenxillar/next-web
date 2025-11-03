use std::{collections::HashMap, sync::Arc};

use next_web_core::traits::filter::{http_filter::HttpFilter, http_filter_chain::HttpFilterChain};


pub trait FilterChainManager
where 
Self: Send + Sync
{
    fn get_filters(&self) -> &mut HashMap<String, Box<dyn HttpFilter>>;

    fn get_chain(&self, chain_name: &str);

    fn has_chains(&self) -> bool;

    fn get_chain_names(&self) -> Vec<&str>;

    fn proxy(&self, original: &dyn HttpFilterChain , chain_name: &str) -> Option<&dyn HttpFilterChain>;

    fn add_filter(&mut self, name: String, filter: Arc<dyn HttpFilter>);

    fn create_chain(&mut self, chain_name: String, chain_definition: String);

    fn create_default_chain(&mut self, chain_name: String);

    fn add_to_chain(&mut self,chain_name: &str, filter_name: String);

    fn set_global_filters(&mut self, global_filter_names: Vec<String>);
}