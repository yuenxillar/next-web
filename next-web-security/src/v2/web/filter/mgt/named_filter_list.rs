use next_web_core::{
    Required,
    http::filter::{HttpFilter, HttpFilterChain},
};

pub trait NamedFilterList
where
    Self: Send + Sync,
    Self: Required<Vec<Box<dyn HttpFilter>>>,
{
    fn name(&self) -> &str;

    fn proxy(&self, filter_chain: &dyn HttpFilterChain) -> Box<dyn HttpFilterChain>;
}
