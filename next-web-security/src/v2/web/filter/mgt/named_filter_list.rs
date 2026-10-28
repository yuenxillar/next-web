use next_web_core::{
    http::filter::{HttpFilter, HttpFilterChain},
    Required,
};

pub trait NamedFilterList
where
    Self: Send + Sync,
    Self: Required<Vec<Box<dyn HttpFilter>>>,
{
    fn name(&self) -> &str;

    fn proxy(&self, filter_chain: &dyn HttpFilterChain) -> Box<dyn HttpFilterChain>;
}
