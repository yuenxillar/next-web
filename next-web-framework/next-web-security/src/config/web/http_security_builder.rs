use crate::core::filter::Filter;

pub trait HttpSecurityBuilder<H>: Send + Sync {
    fn add_filter(&mut self, filter: impl Filter) -> H;
}
