use indexmap::IndexMap;
use next_web_core::traits::filter::{http_filter::HttpFilter, http_filter_chain::HttpFilterChain};

use crate::web::filter::mgt::named_filter_list::NamedFilterList;


pub trait FilterChainManager
where 
Self: Send + Sync
{
    fn get_filters(&mut self) -> &mut IndexMap<String, Box<dyn HttpFilter>>;

    fn get_chain(&self, chain_name: &str) -> Option<&dyn NamedFilterList>;

    fn has_chains(&self) -> bool;

    fn get_chain_names(&self) -> Vec<&str>;

    fn proxy(&self, original: &dyn HttpFilterChain , chain_name: &str) -> Box<dyn HttpFilterChain>;

    fn add_filter(&mut self, name: String, filter: Box<dyn HttpFilter>);

    fn create_chain(&mut self, chain_name: String, chain_definition: String);

    fn create_default_chain(&mut self, chain_name: String);

    fn add_to_chain(&mut self,chain_name: &str, filter_name: String, chain_specific_filter_config: Option<&str>);

    fn set_global_filters(&mut self, global_filter_names: Vec<String>);
}