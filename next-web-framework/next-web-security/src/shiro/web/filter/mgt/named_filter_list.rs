use next_web_core::traits::{
    filter::{http_filter::HttpFilter, http_filter_chain::HttpFilterChain},
    required::Required,
};

pub trait NamedFilterList
where
    Self: Send + Sync,
    Self: Required<Vec<Box<dyn HttpFilter>>>,
{
    fn name(&self) -> &str;

    fn proxy(&self, filter_chain: &dyn HttpFilterChain) -> Box<dyn HttpFilterChain>;
}
