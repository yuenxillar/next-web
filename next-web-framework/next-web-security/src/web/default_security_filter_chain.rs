use crate::web::security_filter_chain::SecurityFilterChain;

#[derive(Clone)]
pub struct DefaultSecurityFilterChain {}

impl SecurityFilterChain for DefaultSecurityFilterChain {
    fn matches(&self, request: &axum::extract::Request) -> bool {
        todo!()
    }

    fn get_filters(&self) -> Vec<std::sync::Arc<dyn crate::core::filter::Filter>> {
        todo!()
    }
}
